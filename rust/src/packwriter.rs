//! Direct git packfile + index writer — bypasses git entirely.
//!
//! Writes both .pack AND .idx with no subprocess in the hot loop.
//! Manual zlib store framing (no flate2) + self-written pack index v2.
//! Only calls `git update-ref` once at the end.

use crc32fast::Hasher as Crc32Hasher;
use pyo3::prelude::*;
use sha1::{Digest, Sha1};
use std::io::Write;
use std::path::PathBuf;

const OBJ_COMMIT: u8 = 1;
const OBJ_TREE: u8 = 2;
const OBJ_BLOB: u8 = 3;

struct ObjEntry {
    sha: [u8; 20],
    crc32: u32,
    offset: u64,
}

#[inline]
fn git_hash(obj_type: &[u8], content: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(obj_type);
    hasher.update(b" ");
    let mut size_buf = [0u8; 20];
    let size_bytes = itoa_buf(content.len() as u64, &mut size_buf);
    hasher.update(size_bytes);
    hasher.update(b"\0");
    hasher.update(content);
    hasher.finalize().into()
}

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

/// Write pack entry with manual zlib store. Returns (CRC32, bytes_written).
#[inline]
fn write_pack_entry(
    w: &mut impl Write,
    pack_hasher: &mut Sha1,
    obj_type: u8,
    data: &[u8],
) -> std::io::Result<(u32, u64)> {
    let mut crc = Crc32Hasher::new();
    let mut written: u64 = 0;

    // Varint header
    let size = data.len();
    let mut hdr = [0u8; 10];
    let mut hdr_len = 0;
    hdr[0] = (obj_type << 4) | (size as u8 & 0x0f);
    let mut remaining = size >> 4;
    if remaining > 0 { hdr[0] |= 0x80; }
    hdr_len += 1;
    while remaining > 0 {
        hdr[hdr_len] = (remaining & 0x7f) as u8;
        remaining >>= 7;
        if remaining > 0 { hdr[hdr_len] |= 0x80; }
        hdr_len += 1;
    }
    w.write_all(&hdr[..hdr_len])?;
    pack_hasher.update(&hdr[..hdr_len]);
    crc.update(&hdr[..hdr_len]);
    written += hdr_len as u64;

    // Zlib store: header + single stored block + adler32
    let len = data.len() as u16;
    let nlen = !len;
    let zlib_framing = [
        0x78, 0x01, // zlib header (deflate, 32K window)
        0x01,       // BFINAL=1, BTYPE=00 (stored)
        (len & 0xff) as u8, (len >> 8) as u8,
        (nlen & 0xff) as u8, (nlen >> 8) as u8,
    ];
    w.write_all(&zlib_framing)?;
    pack_hasher.update(&zlib_framing);
    crc.update(&zlib_framing);
    written += 7;

    w.write_all(data)?;
    pack_hasher.update(data);
    crc.update(data);
    written += data.len() as u64;

    let cksum = adler32(data).to_be_bytes();
    w.write_all(&cksum)?;
    pack_hasher.update(&cksum);
    crc.update(&cksum);
    written += 4;

    Ok((crc.finalize(), written))
}

/// Write pack index v2 file.
fn write_idx(
    idx_path: &std::path::Path,
    entries: &mut [ObjEntry],
    pack_checksum: &[u8; 20],
) -> std::io::Result<()> {
    entries.sort_unstable_by(|a, b| a.sha.cmp(&b.sha));

    let file = std::fs::File::create(idx_path)?;
    let mut w = std::io::BufWriter::with_capacity(1 << 20, file);
    let mut idx_hasher = Sha1::new();

    macro_rules! wh {
        ($data:expr) => {{ w.write_all($data)?; idx_hasher.update($data); }};
    }

    // Header
    wh!(&[0xff, 0x74, 0x4f, 0x63]);
    wh!(&2u32.to_be_bytes());

    // Fan-out
    let mut fanout = [0u32; 256];
    for e in entries.iter() { fanout[e.sha[0] as usize] += 1; }
    for i in 1..256 { fanout[i] += fanout[i - 1]; }
    for c in &fanout { wh!(&c.to_be_bytes()); }

    // SHAs
    for e in entries.iter() { wh!(&e.sha); }

    // CRC32s
    for e in entries.iter() { wh!(&e.crc32.to_be_bytes()); }

    // Offsets
    let mut large_offsets: Vec<u64> = Vec::new();
    for e in entries.iter() {
        if e.offset > 0x7fffffff {
            let idx = large_offsets.len() as u32;
            wh!(&(0x80000000 | idx).to_be_bytes());
            large_offsets.push(e.offset);
        } else {
            wh!(&(e.offset as u32).to_be_bytes());
        }
    }
    for &off in &large_offsets { wh!(&off.to_be_bytes()); }

    // Checksums
    wh!(pack_checksum);
    let idx_checksum: [u8; 20] = idx_hasher.finalize().into();
    w.write_all(&idx_checksum)?;
    w.flush()?;
    Ok(())
}

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
    let total_objects = (batches * commits_per_batch) as usize + 2;
    let pack_dir = PathBuf::from(repo_path).join(".git/objects/pack");
    let pack_name = format!("pack-{:016x}", base_batch_tag);
    let pack_path = pack_dir.join(format!("{pack_name}.pack"));
    let idx_path = pack_dir.join(format!("{pack_name}.idx"));

    let t0 = std::time::Instant::now();

    let mut entries: Vec<ObjEntry> = Vec::with_capacity(total_objects);
    let file = std::fs::File::create(&pack_path)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("create pack: {e}")))?;
    let mut w = std::io::BufWriter::with_capacity(2 << 20, file);
    let mut pack_hasher = Sha1::new();

    // Pack header
    let mut header = [0u8; 12];
    header[0..4].copy_from_slice(b"PACK");
    header[4..8].copy_from_slice(&2u32.to_be_bytes());
    header[8..12].copy_from_slice(&(total_objects as u32).to_be_bytes());
    w.write_all(&header).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    pack_hasher.update(&header);
    let mut current_offset: u64 = 12;

    // Blob
    let blob_content = b"h\n";
    let blob_sha = git_hash(b"blob", blob_content);
    let (crc, bytes) = write_pack_entry(&mut w, &mut pack_hasher, OBJ_BLOB, blob_content)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    entries.push(ObjEntry { sha: blob_sha, crc32: crc, offset: current_offset });
    current_offset += bytes;

    // Tree
    let mut tree_content = Vec::with_capacity(32);
    tree_content.extend_from_slice(b"100644 s\0");
    tree_content.extend_from_slice(&blob_sha);
    let tree_sha = git_hash(b"tree", &tree_content);
    let mut tree_hex = [0u8; 40];
    sha_hex_into(&tree_sha, &mut tree_hex);
    let (crc, bytes) = write_pack_entry(&mut w, &mut pack_hasher, OBJ_TREE, &tree_content)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    entries.push(ObjEntry { sha: tree_sha, crc32: crc, offset: current_offset });
    current_offset += bytes;

    // Commits
    let mut prev_sha: Option<[u8; 20]> = parent_sha.map(|s| {
        let mut sha = [0u8; 20];
        for i in 0..20 {
            sha[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        sha
    });

    let mut total_commits: u64 = 0;
    let mut last_sha = [0u8; 20];
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

            let commit_sha = git_hash(b"commit", &commit_buf);
            let (crc, bytes) = write_pack_entry(&mut w, &mut pack_hasher, OBJ_COMMIT, &commit_buf)
                .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
            entries.push(ObjEntry { sha: commit_sha, crc32: crc, offset: current_offset });
            current_offset += bytes;

            prev_sha = Some(commit_sha);
            last_sha = commit_sha;
        }
        total_commits += commits_per_batch;
    }

    // Pack checksum
    let pack_checksum: [u8; 20] = pack_hasher.finalize().into();
    w.write_all(&pack_checksum).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    w.flush().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    drop(w);

    let write_time = t0.elapsed().as_secs_f64();
    eprintln!(
        "[direct-pack] pack write: {:.1} ms ({} commits, {:.1} MiB)",
        write_time * 1000.0, total_commits,
        std::fs::metadata(&pack_path).map(|m| m.len()).unwrap_or(0) as f64 / 1048576.0,
    );

    // Write idx
    let t_idx = std::time::Instant::now();
    write_idx(&idx_path, &mut entries, &pack_checksum)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("write idx: {e}")))?;
    eprintln!("[direct-pack] idx write: {:.1} ms", t_idx.elapsed().as_secs_f64() * 1000.0);

    // Rename to pack-<checksum>.{pack,idx}
    let mut cksum_hex = [0u8; 40];
    sha_hex_into(&pack_checksum, &mut cksum_hex);
    let cksum_str = std::str::from_utf8(&cksum_hex).unwrap();
    let final_pack = pack_dir.join(format!("pack-{cksum_str}.pack"));
    let final_idx = pack_dir.join(format!("pack-{cksum_str}.idx"));
    std::fs::rename(&pack_path, &final_pack)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("rename pack: {e}")))?;
    std::fs::rename(&idx_path, &final_idx)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("rename idx: {e}")))?;

    // Update ref
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
        return Err(pyo3::exceptions::PyOSError::new_err(format!("git update-ref failed: {stderr}")));
    }

    let total_time = t0.elapsed().as_secs_f64();
    eprintln!("[direct-pack] total: {:.1} ms", total_time * 1000.0);

    Ok((total_commits, total_time))
}
