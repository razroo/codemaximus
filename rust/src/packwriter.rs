//! Direct git packfile writer — bypasses `git fast-import` entirely.
//!
//! Creates commit objects in a tight loop, computes SHA-1 hashes, and writes
//! them directly into a `.pack` file with manual zlib store framing (no
//! compression library overhead). Then calls `git index-pack` once to finalize.

use pyo3::prelude::*;
use sha1::{Digest, Sha1};
use std::io::Write;
use std::path::PathBuf;

const OBJ_COMMIT: u8 = 1;
const OBJ_TREE: u8 = 2;
const OBJ_BLOB: u8 = 3;

/// Compute git object hash: SHA-1("type size\0content").
#[inline]
fn git_hash(obj_type: &[u8], content: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(obj_type);
    hasher.update(b" ");
    // Write size as ASCII digits
    let mut size_buf = [0u8; 20];
    let size_bytes = itoa_buf(content.len() as u64, &mut size_buf);
    hasher.update(size_bytes);
    hasher.update(b"\0");
    hasher.update(content);
    hasher.finalize().into()
}

/// Encode 20-byte SHA as 40-char hex into a provided buffer.
#[inline]
fn sha_hex_into(sha: &[u8; 20], out: &mut [u8; 40]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for i in 0..20 {
        out[i * 2] = HEX[(sha[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(sha[i] & 0x0f) as usize];
    }
}

#[inline(always)]
fn itoa_buf(n: u64, buf: &mut [u8; 20]) -> &[u8] {
    let mut i = 20;
    let mut val = n;
    if val == 0 {
        i -= 1;
        buf[i] = b'0';
        return &buf[i..];
    }
    while val > 0 {
        i -= 1;
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    &buf[i..]
}

/// Compute Adler-32 checksum.
#[inline]
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Write zlib "stored" format manually — no compression, just framing.
/// Much faster than going through flate2 for small data.
#[inline]
fn write_zlib_store(w: &mut impl Write, hasher: &mut Sha1, data: &[u8]) -> std::io::Result<()> {
    // Zlib header: CMF=0x78 (deflate, 32K window), FLG=0x01
    let header = [0x78u8, 0x01];
    w.write_all(&header)?;
    hasher.update(&header);

    // Deflate stored block: BFINAL=1, BTYPE=00
    let len = data.len() as u16;
    let nlen = !len;
    let block_header = [
        0x01, // BFINAL=1, BTYPE=00 (stored)
        (len & 0xff) as u8,
        (len >> 8) as u8,
        (nlen & 0xff) as u8,
        (nlen >> 8) as u8,
    ];
    w.write_all(&block_header)?;
    hasher.update(&block_header);

    // Raw data
    w.write_all(data)?;
    hasher.update(data);

    // Adler-32 checksum (big-endian)
    let cksum = adler32(data);
    let cksum_bytes = cksum.to_be_bytes();
    w.write_all(&cksum_bytes)?;
    hasher.update(&cksum_bytes);

    Ok(())
}

/// Write pack entry varint header + zlib-stored data.
#[inline]
fn write_pack_entry(
    w: &mut impl Write,
    hasher: &mut Sha1,
    obj_type: u8,
    data: &[u8],
) -> std::io::Result<()> {
    // Type+size varint
    let size = data.len();
    let mut hdr = [0u8; 10];
    let mut hdr_len = 0;
    hdr[0] = (obj_type << 4) | (size as u8 & 0x0f);
    let mut remaining = size >> 4;
    if remaining > 0 {
        hdr[0] |= 0x80;
    }
    hdr_len += 1;
    while remaining > 0 {
        hdr[hdr_len] = (remaining & 0x7f) as u8;
        remaining >>= 7;
        if remaining > 0 {
            hdr[hdr_len] |= 0x80;
        }
        hdr_len += 1;
    }
    w.write_all(&hdr[..hdr_len])?;
    hasher.update(&hdr[..hdr_len]);

    write_zlib_store(w, hasher, data)
}

/// Direct packfile writer: creates commits without any git subprocess.
///
/// Writes a .pack file, calls `git index-pack` and `git update-ref`.
/// Returns (total_commits, wall_time_secs).
#[pyfunction]
#[pyo3(signature = (repo_path, branch, batches, commits_per_batch, parent_sha, base_batch_tag, base_ts))]
pub fn hyperdrive_direct(
    repo_path: &str,
    branch: &str,
    batches: u64,
    commits_per_batch: u64,
    parent_sha: Option<&str>,
    base_batch_tag: u64,
    base_ts: u64,
) -> PyResult<(u64, f64)> {
    let total_objects = (batches * commits_per_batch) + 2; // +2 for blob and tree
    let pack_dir = PathBuf::from(repo_path).join(".git/objects/pack");
    let pack_name = format!("pack-hyperdrive-{}.pack", base_batch_tag);
    let pack_path = pack_dir.join(&pack_name);

    let t0 = std::time::Instant::now();

    let file = std::fs::File::create(&pack_path)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("create pack: {e}")))?;
    let mut w = std::io::BufWriter::with_capacity(1 << 20, file);
    let mut pack_hasher = Sha1::new();

    // Pack header
    let mut header = [0u8; 12];
    header[0..4].copy_from_slice(b"PACK");
    header[4..8].copy_from_slice(&2u32.to_be_bytes());
    header[8..12].copy_from_slice(&(total_objects as u32).to_be_bytes());
    w.write_all(&header)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    pack_hasher.update(&header);

    // Blob: "h\n"
    let blob_content = b"h\n";
    let blob_sha = git_hash(b"blob", blob_content);
    write_pack_entry(&mut w, &mut pack_hasher, OBJ_BLOB, blob_content)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

    // Tree: single entry "100644 s\0<blob_sha>"
    let mut tree_content = Vec::with_capacity(32);
    tree_content.extend_from_slice(b"100644 s\0");
    tree_content.extend_from_slice(&blob_sha);
    let tree_sha = git_hash(b"tree", &tree_content);
    let mut tree_hex = [0u8; 40];
    sha_hex_into(&tree_sha, &mut tree_hex);
    write_pack_entry(&mut w, &mut pack_hasher, OBJ_TREE, &tree_content)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

    // Parse parent SHA
    let mut prev_sha: Option<[u8; 20]> = parent_sha.map(|s| {
        let mut sha = [0u8; 20];
        for i in 0..20 {
            sha[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        sha
    });

    let mut total_commits: u64 = 0;
    let mut last_sha = [0u8; 20];
    // Reusable buffers
    let mut commit_buf = Vec::with_capacity(256);
    let mut parent_hex = [0u8; 40];

    for batch_idx in 0..batches {
        let batch_tag = base_batch_tag + batch_idx;
        let batch_base_ts = base_ts + total_commits;

        let mut tag_buf = [0u8; 20];
        let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();

        for i in 0..commits_per_batch {
            let commit_num = i + 1;
            let ts = batch_base_ts + i;

            // Build commit content into reusable buffer
            commit_buf.clear();
            commit_buf.extend_from_slice(b"tree ");
            commit_buf.extend_from_slice(&tree_hex);
            commit_buf.push(b'\n');

            if let Some(ref ps) = prev_sha {
                sha_hex_into(ps, &mut parent_hex);
                commit_buf.extend_from_slice(b"parent ");
                commit_buf.extend_from_slice(&parent_hex);
                commit_buf.push(b'\n');
            }

            commit_buf.extend_from_slice(b"author cm <cm@h> ");
            let mut ts_buf = [0u8; 20];
            let ts_b = itoa_buf(ts, &mut ts_buf);
            commit_buf.extend_from_slice(ts_b);
            commit_buf.extend_from_slice(b" +0000\ncommitter cm <cm@h> ");
            commit_buf.extend_from_slice(ts_b);
            commit_buf.extend_from_slice(b" +0000\n\n");
            commit_buf.extend_from_slice(&tag_bytes);
            commit_buf.push(b'.');
            let mut cn_buf = [0u8; 20];
            let cn_b = itoa_buf(commit_num, &mut cn_buf);
            commit_buf.extend_from_slice(cn_b);

            // Hash
            let commit_sha = git_hash(b"commit", &commit_buf);
            // Write pack entry
            write_pack_entry(&mut w, &mut pack_hasher, OBJ_COMMIT, &commit_buf)
                .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

            prev_sha = Some(commit_sha);
            last_sha = commit_sha;
        }

        total_commits += commits_per_batch;
    }

    // Pack checksum
    let checksum: [u8; 20] = pack_hasher.finalize().into();
    w.write_all(&checksum)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    w.flush()
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    drop(w);

    let write_time = t0.elapsed().as_secs_f64();
    eprintln!(
        "[direct-pack] pack write: {:.1} ms ({} commits, {:.1} MiB)",
        write_time * 1000.0,
        total_commits,
        std::fs::metadata(&pack_path).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0 / 1024.0,
    );

    // Index the pack
    let t_idx = std::time::Instant::now();
    let idx_result = std::process::Command::new("git")
        .args(["index-pack", pack_path.to_str().unwrap()])
        .current_dir(repo_path)
        .output()
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("index-pack: {e}")))?;

    if !idx_result.status.success() {
        let stderr = String::from_utf8_lossy(&idx_result.stderr);
        return Err(pyo3::exceptions::PyOSError::new_err(
            format!("git index-pack failed: {stderr}"),
        ));
    }
    eprintln!("[direct-pack] index-pack: {:.1} ms", t_idx.elapsed().as_secs_f64() * 1000.0);

    // Update branch ref
    let mut ref_hex = [0u8; 40];
    sha_hex_into(&last_sha, &mut ref_hex);
    let ref_sha_str = std::str::from_utf8(&ref_hex).unwrap();
    let ref_result = std::process::Command::new("git")
        .args(["update-ref", &format!("refs/heads/{branch}"), ref_sha_str])
        .current_dir(repo_path)
        .output()
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("update-ref: {e}")))?;

    if !ref_result.status.success() {
        let stderr = String::from_utf8_lossy(&ref_result.stderr);
        return Err(pyo3::exceptions::PyOSError::new_err(
            format!("git update-ref failed: {stderr}"),
        ));
    }

    let total_time = t0.elapsed().as_secs_f64();
    eprintln!("[direct-pack] total: {:.1} ms", total_time * 1000.0);

    Ok((total_commits, total_time))
}
