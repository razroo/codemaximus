//! Fast-import stream builder for hyperdrive mode.
//!
//! v3 — blob/tree reuse: creates one blob object, every commit references it
//! by mark so git reuses the blob AND tree (1 object/commit instead of 3).
//! Shortened identity/path/message cuts stream size ~60%.

use pyo3::prelude::*;
use std::io::Write;

/// Fast integer-to-stack-buffer conversion (avoids format! allocation per number).
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

// Compact identity: short name saves ~94 bytes/commit (author + committer)
const AUTHOR_PRE: &[u8] = b"author cm <cm@h> ";
const COMMITTER_PRE: &[u8] = b"committer cm <cm@h> ";
const TZ_SUF: &[u8] = b" +0000\n";
// Short file path: 1 char instead of 28
const FILE_PATH: &[u8] = b"s";
// Blob content (fixed — reused across all commits)
const BLOB_CONTENT: &[u8] = b"h\n";

/// Helper macro — maps io::Error to PyOSError.
macro_rules! w {
    ($writer:expr, $data:expr) => {
        $writer.write_all($data).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))
    };
}

/// Write the shared blob that all commits will reference.
/// Uses mark :1, returns Ok(()).
#[inline]
fn write_blob<W: Write>(w: &mut W) -> std::io::Result<()> {
    w.write_all(b"blob\nmark :1\ndata ")?;
    let mut buf = [0u8; 20];
    let len_b = itoa_buf(BLOB_CONTENT.len() as u64, &mut buf);
    w.write_all(len_b)?;
    w.write_all(b"\n")?;
    w.write_all(BLOB_CONTENT)?;
    w.write_all(b"\n")?;
    Ok(())
}

/// Write a single commit entry referencing the shared blob at mark :1.
/// `mark`: this commit's mark number (must be >= 2, since :1 is the blob)
/// `parent`: either `from <sha>\n` or `from :<mark>\n` bytes, or empty for root
#[inline]
fn write_commit<W: Write>(
    w: &mut W,
    ref_bytes: &[u8],
    mark: u64,
    ts: u64,
    parent_line: &[u8],
    msg: &[u8],
) -> std::io::Result<()> {
    let mut mark_buf = [0u8; 20];
    let mark_b = itoa_buf(mark, &mut mark_buf);
    let mut ts_buf = [0u8; 20];
    let ts_b = itoa_buf(ts, &mut ts_buf);
    let mut len_buf = [0u8; 20];
    let len_b = itoa_buf(msg.len() as u64, &mut len_buf);

    w.write_all(ref_bytes)?;
    w.write_all(b"mark :")?;
    w.write_all(mark_b)?;
    w.write_all(b"\n")?;
    w.write_all(AUTHOR_PRE)?;
    w.write_all(ts_b)?;
    w.write_all(TZ_SUF)?;
    w.write_all(COMMITTER_PRE)?;
    w.write_all(ts_b)?;
    w.write_all(TZ_SUF)?;
    w.write_all(b"data ")?;
    w.write_all(len_b)?;
    w.write_all(b"\n")?;
    w.write_all(msg)?;
    w.write_all(b"\n")?;
    w.write_all(parent_line)?;
    // Reference shared blob by mark instead of inline data
    w.write_all(b"M 100644 :1 ")?;
    w.write_all(FILE_PATH)?;
    w.write_all(b"\n")?;
    Ok(())
}

/// Build a complete `git fast-import` stream as bytes (v3: blob reuse + compact).
#[pyfunction]
#[pyo3(signature = (branch, n, parent_sha, batch_tag, base_ts))]
pub fn build_fast_import_stream(
    branch: &str,
    n: u64,
    parent_sha: Option<&str>,
    batch_tag: u64,
    base_ts: u64,
) -> Vec<u8> {
    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();

    let mut tag_buf = [0u8; 20];
    let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();
    let mut n_buf = [0u8; 20];
    let n_bytes = itoa_buf(n, &mut n_buf).to_vec();

    // ~120 bytes per commit with compact format
    let mut buf: Vec<u8> = Vec::with_capacity(64 + (n as usize) * 130);

    // Write shared blob as mark :1
    write_blob(&mut buf).unwrap();

    for i in 0..n {
        // Marks start at 2 (1 is the blob)
        let mark = i + 2;
        let ts = base_ts + i;
        let commit_num = i + 1;

        // Compact message: "{tag}.{num}"
        let mut cn_buf = [0u8; 20];
        let cn_b = itoa_buf(commit_num, &mut cn_buf);
        let mut msg: Vec<u8> = Vec::with_capacity(24);
        msg.extend_from_slice(&tag_bytes);
        msg.push(b'.');
        msg.extend_from_slice(cn_b);

        // Parent line
        let parent_line: Vec<u8> = if i == 0 {
            if let Some(sha) = parent_sha {
                format!("from {sha}\n").into_bytes()
            } else {
                Vec::new()
            }
        } else {
            let mut pm_buf = [0u8; 20];
            let pm_b = itoa_buf(mark - 1, &mut pm_buf);
            let mut v = Vec::with_capacity(16);
            v.extend_from_slice(b"from :");
            v.extend_from_slice(pm_b);
            v.push(b'\n');
            v
        };

        write_commit(&mut buf, ref_bytes, mark, ts, &parent_line, &msg).unwrap();
    }

    buf
}

/// Build the fast-import stream and write it directly to a file descriptor (v3).
#[pyfunction]
#[pyo3(signature = (fd, branch, n, parent_sha, batch_tag, base_ts))]
pub fn stream_fast_import_to_fd(
    fd: i32,
    branch: &str,
    n: u64,
    parent_sha: Option<&str>,
    batch_tag: u64,
    base_ts: u64,
) -> PyResult<u64> {
    use std::os::unix::io::FromRawFd;

    let raw_file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut w = std::io::BufWriter::with_capacity(1 << 20, raw_file);

    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();

    let mut tag_buf = [0u8; 20];
    let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();

    // Write shared blob
    write_blob(&mut w).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

    for i in 0..n {
        let mark = i + 2;
        let ts = base_ts + i;
        let commit_num = i + 1;

        let mut cn_buf = [0u8; 20];
        let cn_b = itoa_buf(commit_num, &mut cn_buf);
        let mut msg: Vec<u8> = Vec::with_capacity(24);
        msg.extend_from_slice(&tag_bytes);
        msg.push(b'.');
        msg.extend_from_slice(cn_b);

        let parent_line: Vec<u8> = if i == 0 {
            if let Some(sha) = parent_sha {
                format!("from {sha}\n").into_bytes()
            } else {
                Vec::new()
            }
        } else {
            let mut pm_buf = [0u8; 20];
            let pm_b = itoa_buf(mark - 1, &mut pm_buf);
            let mut v = Vec::with_capacity(16);
            v.extend_from_slice(b"from :");
            v.extend_from_slice(pm_b);
            v.push(b'\n');
            v
        };

        write_commit(&mut w, ref_bytes, mark, ts, &parent_line, &msg)
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    }

    w.flush().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    let inner = w.into_inner().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    std::mem::forget(inner);

    Ok(n)
}

/// Stream multiple batches through a single fd / single fast-import process (v3).
///
/// Produces ONE pack file. Uses shared blob (mark :1) so git creates only
/// 1 blob + 1 tree for the entire run — every commit is just a commit object.
#[pyfunction]
#[pyo3(signature = (fd, branch, batches, commits_per_batch, parent_sha, base_batch_tag, base_ts))]
pub fn stream_multi_batch_to_fd(
    fd: i32,
    branch: &str,
    batches: u64,
    commits_per_batch: u64,
    parent_sha: Option<&str>,
    base_batch_tag: u64,
    base_ts: u64,
) -> PyResult<u64> {
    use std::os::unix::io::FromRawFd;

    let raw_file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut w = std::io::BufWriter::with_capacity(1 << 20, raw_file);

    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();

    // Write shared blob as mark :1
    write_blob(&mut w).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

    // Commit marks start at 2
    let mut global_mark: u64 = 1;
    let mut total_commits: u64 = 0;
    let mut prev_mark: Option<u64> = None;

    for batch_idx in 0..batches {
        let batch_tag = base_batch_tag + batch_idx;
        let batch_base_ts = base_ts + total_commits;

        let mut tag_buf = [0u8; 20];
        let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();

        for i in 0..commits_per_batch {
            global_mark += 1;
            let mark = global_mark;
            let commit_num = i + 1;
            let ts = batch_base_ts + i;

            // Compact message: "{tag}.{num}"
            let mut cn_buf = [0u8; 20];
            let cn_b = itoa_buf(commit_num, &mut cn_buf);
            let mut msg: Vec<u8> = Vec::with_capacity(24);
            msg.extend_from_slice(&tag_bytes);
            msg.push(b'.');
            msg.extend_from_slice(cn_b);

            // Parent line
            let parent_line: Vec<u8> = if i == 0 && batch_idx == 0 {
                if let Some(sha) = parent_sha {
                    format!("from {sha}\n").into_bytes()
                } else {
                    Vec::new()
                }
            } else if let Some(pm) = prev_mark {
                let mut pm_buf = [0u8; 20];
                let pm_b = itoa_buf(pm, &mut pm_buf);
                let mut v = Vec::with_capacity(16);
                v.extend_from_slice(b"from :");
                v.extend_from_slice(pm_b);
                v.push(b'\n');
                v
            } else {
                Vec::new()
            };

            write_commit(&mut w, ref_bytes, mark, ts, &parent_line, &msg)
                .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;

            prev_mark = Some(mark);
        }

        total_commits += commits_per_batch;
    }

    w.flush().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    let inner = w.into_inner().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    std::mem::forget(inner);

    Ok(total_commits)
}
