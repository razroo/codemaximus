//! Fast-import stream builder for hyperdrive mode.
//!
//! Builds the entire `git fast-import` byte stream in Rust, avoiding
//! millions of Python string-format + encode calls. Uses pre-computed
//! byte templates and integer-to-bytes via itoa to minimize allocations.

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

/// Build a complete `git fast-import` stream as bytes.
#[pyfunction]
#[pyo3(signature = (branch, n, parent_sha, batch_tag, base_ts))]
pub fn build_fast_import_stream(
    branch: &str,
    n: u64,
    parent_sha: Option<&str>,
    batch_tag: u64,
    base_ts: u64,
) -> Vec<u8> {
    let file_path = b"slop/hyperdrive_signal.txt";
    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();

    // Pre-build the static identity line portions
    let author_pre = b"author Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let committer_pre = b"committer Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let tz_suf = b" +0000\n";

    // Pre-format the batch_tag and n as bytes
    let mut tag_buf = [0u8; 20];
    let tag_bytes = itoa_buf(batch_tag, &mut tag_buf);
    let mut n_buf = [0u8; 20];
    let n_bytes = itoa_buf(n, &mut n_buf);

    // ~160 bytes per commit (measured)
    let mut buf: Vec<u8> = Vec::with_capacity((n as usize) * 170);
    let mut mark_buf = [0u8; 20];
    let mut ts_buf = [0u8; 20];

    for i in 0..n {
        let mark = i + 1;
        let ts = base_ts + i;
        let mark_b = itoa_buf(mark, &mut mark_buf);
        let ts_b = itoa_buf(ts, &mut ts_buf);

        // Build commit message: "hyperdrive batch {tag} commit {mark}/{n}"
        // and body: "{tag}:{mark}:{ts}\n" — into a small stack buffer
        let mut msg_vec: Vec<u8> = Vec::with_capacity(64);
        msg_vec.extend_from_slice(b"hyperdrive batch ");
        msg_vec.extend_from_slice(tag_bytes);
        msg_vec.extend_from_slice(b" commit ");
        msg_vec.extend_from_slice(mark_b);
        msg_vec.push(b'/');
        msg_vec.extend_from_slice(n_bytes);

        let mut body_vec: Vec<u8> = Vec::with_capacity(32);
        body_vec.extend_from_slice(tag_bytes);
        body_vec.push(b':');
        body_vec.extend_from_slice(mark_b);
        body_vec.push(b':');
        body_vec.extend_from_slice(ts_b);
        body_vec.push(b'\n');

        // commit ref + mark
        buf.extend_from_slice(ref_bytes);
        buf.extend_from_slice(b"mark :");
        buf.extend_from_slice(mark_b);
        buf.push(b'\n');

        // author / committer (no format!, just byte concat)
        buf.extend_from_slice(author_pre);
        buf.extend_from_slice(ts_b);
        buf.extend_from_slice(tz_suf);
        buf.extend_from_slice(committer_pre);
        buf.extend_from_slice(ts_b);
        buf.extend_from_slice(tz_suf);

        // data <len>\n<msg>\n
        let mut len_buf = [0u8; 20];
        let len_b = itoa_buf(msg_vec.len() as u64, &mut len_buf);
        buf.extend_from_slice(b"data ");
        buf.extend_from_slice(len_b);
        buf.push(b'\n');
        buf.extend_from_slice(&msg_vec);
        buf.push(b'\n');

        // parent linkage
        if i == 0 {
            if let Some(sha) = parent_sha {
                buf.extend_from_slice(b"from ");
                buf.extend_from_slice(sha.as_bytes());
                buf.push(b'\n');
            }
        } else {
            buf.extend_from_slice(b"from :");
            let mut pm_buf = [0u8; 20];
            let pm_b = itoa_buf(mark - 1, &mut pm_buf);
            buf.extend_from_slice(pm_b);
            buf.push(b'\n');
        }

        // file modification
        buf.extend_from_slice(b"M 100644 inline ");
        buf.extend_from_slice(file_path);
        buf.push(b'\n');
        let mut blen_buf = [0u8; 20];
        let blen_b = itoa_buf(body_vec.len() as u64, &mut blen_buf);
        buf.extend_from_slice(b"data ");
        buf.extend_from_slice(blen_b);
        buf.push(b'\n');
        buf.extend_from_slice(&body_vec);
    }

    buf
}

/// Helper macro — maps io::Error to PyOSError.
macro_rules! w {
    ($writer:expr, $data:expr) => {
        $writer.write_all($data).map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))
    };
}

/// Build the fast-import stream and write it directly to a file descriptor.
///
/// Avoids allocating the entire stream in memory — pipes directly into
/// `git fast-import` stdin. Returns n (number of commits written).
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
    let mut w = std::io::BufWriter::with_capacity(1 << 20, raw_file); // 1 MiB buffer

    let file_path = b"slop/hyperdrive_signal.txt";
    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();
    let author_pre = b"author Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let committer_pre = b"committer Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let tz_suf = b" +0000\n";

    let mut tag_buf = [0u8; 20];
    let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();
    let mut n_buf_s = [0u8; 20];
    let n_bytes = itoa_buf(n, &mut n_buf_s).to_vec();

    let mut mark_buf = [0u8; 20];
    let mut ts_buf = [0u8; 20];

    for i in 0..n {
        let mark = i + 1;
        let ts = base_ts + i;
        let mark_b = itoa_buf(mark, &mut mark_buf);
        let ts_b = itoa_buf(ts, &mut ts_buf);

        // Build message + body
        let mut msg: Vec<u8> = Vec::with_capacity(64);
        msg.extend_from_slice(b"hyperdrive batch ");
        msg.extend_from_slice(&tag_bytes);
        msg.extend_from_slice(b" commit ");
        msg.extend_from_slice(mark_b);
        msg.push(b'/');
        msg.extend_from_slice(&n_bytes);

        let mut body: Vec<u8> = Vec::with_capacity(32);
        body.extend_from_slice(&tag_bytes);
        body.push(b':');
        body.extend_from_slice(mark_b);
        body.push(b':');
        body.extend_from_slice(ts_b);
        body.push(b'\n');

        w!(w, ref_bytes)?;
        w!(w, b"mark :")?;
        w!(w, mark_b)?;
        w!(w, b"\n")?;

        w!(w, author_pre)?;
        w!(w, ts_b)?;
        w!(w, tz_suf)?;
        w!(w, committer_pre)?;
        w!(w, ts_b)?;
        w!(w, tz_suf)?;

        let mut len_buf = [0u8; 20];
        let len_b = itoa_buf(msg.len() as u64, &mut len_buf);
        w!(w, b"data ")?;
        w!(w, len_b)?;
        w!(w, b"\n")?;
        w!(w, &msg)?;
        w!(w, b"\n")?;

        if i == 0 {
            if let Some(sha) = parent_sha {
                w!(w, b"from ")?;
                w!(w, sha.as_bytes())?;
                w!(w, b"\n")?;
            }
        } else {
            let mut pm_buf = [0u8; 20];
            let pm_b = itoa_buf(mark - 1, &mut pm_buf);
            w!(w, b"from :")?;
            w!(w, pm_b)?;
            w!(w, b"\n")?;
        }

        w!(w, b"M 100644 inline ")?;
        w!(w, file_path)?;
        w!(w, b"\n")?;
        let mut blen_buf = [0u8; 20];
        let blen_b = itoa_buf(body.len() as u64, &mut blen_buf);
        w!(w, b"data ")?;
        w!(w, blen_b)?;
        w!(w, b"\n")?;
        w!(w, &body)?;
    }

    w.flush().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    let inner = w.into_inner().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    std::mem::forget(inner);

    Ok(n)
}

/// Stream multiple batches of commits through a single fd in one call.
///
/// This produces ONE pack file instead of N, eliminating the need for
/// post-run `git repack`. Each batch uses marks offset by batch index
/// so they don't collide within the single fast-import session.
///
/// `batches`: number of batches to stream
/// `commits_per_batch`: commits in each batch
/// `base_batch_tag`: tag for first batch (increments per batch)
///
/// The first batch uses `parent_sha` as its parent; subsequent batches
/// chain from the last mark of the previous batch.
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

    let file_path = b"slop/hyperdrive_signal.txt";
    let ref_line = format!("commit refs/heads/{branch}\n");
    let ref_bytes = ref_line.as_bytes();
    let author_pre = b"author Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let committer_pre = b"committer Codemaximus Hyperdrive <hyperdrive@codemaximus.local> ";
    let tz_suf = b" +0000\n";

    // Global mark counter — must be unique across all batches in one fast-import session
    let mut global_mark: u64 = 0;
    let mut total_commits: u64 = 0;
    // Track the previous commit's mark for parent chaining between batches
    let mut prev_mark: Option<u64> = None;

    for batch_idx in 0..batches {
        let batch_tag = base_batch_tag + batch_idx;
        let batch_base_ts = base_ts + total_commits;

        let mut tag_buf = [0u8; 20];
        let tag_bytes = itoa_buf(batch_tag, &mut tag_buf).to_vec();
        let mut n_buf = [0u8; 20];
        let n_bytes = itoa_buf(commits_per_batch, &mut n_buf).to_vec();

        for i in 0..commits_per_batch {
            global_mark += 1;
            let mark = global_mark;
            let commit_num = i + 1;
            let ts = batch_base_ts + i;

            let mut mark_buf = [0u8; 20];
            let mark_b = itoa_buf(mark, &mut mark_buf);
            let mut ts_buf = [0u8; 20];
            let ts_b = itoa_buf(ts, &mut ts_buf);
            let mut cn_buf = [0u8; 20];
            let cn_b = itoa_buf(commit_num, &mut cn_buf);

            // Message: "hyperdrive batch {tag} commit {num}/{total}"
            let mut msg: Vec<u8> = Vec::with_capacity(64);
            msg.extend_from_slice(b"hyperdrive batch ");
            msg.extend_from_slice(&tag_bytes);
            msg.extend_from_slice(b" commit ");
            msg.extend_from_slice(cn_b);
            msg.push(b'/');
            msg.extend_from_slice(&n_bytes);

            // Body: "{tag}:{num}:{ts}\n"
            let mut body: Vec<u8> = Vec::with_capacity(32);
            body.extend_from_slice(&tag_bytes);
            body.push(b':');
            body.extend_from_slice(cn_b);
            body.push(b':');
            body.extend_from_slice(ts_b);
            body.push(b'\n');

            w!(w, ref_bytes)?;
            w!(w, b"mark :")?;
            w!(w, mark_b)?;
            w!(w, b"\n")?;

            w!(w, author_pre)?;
            w!(w, ts_b)?;
            w!(w, tz_suf)?;
            w!(w, committer_pre)?;
            w!(w, ts_b)?;
            w!(w, tz_suf)?;

            let mut len_buf = [0u8; 20];
            let len_b = itoa_buf(msg.len() as u64, &mut len_buf);
            w!(w, b"data ")?;
            w!(w, len_b)?;
            w!(w, b"\n")?;
            w!(w, &msg)?;
            w!(w, b"\n")?;

            // Parent: first commit of first batch uses parent_sha,
            // everything else chains from previous mark
            if i == 0 && batch_idx == 0 {
                if let Some(sha) = parent_sha {
                    w!(w, b"from ")?;
                    w!(w, sha.as_bytes())?;
                    w!(w, b"\n")?;
                }
            } else if let Some(pm) = prev_mark {
                let mut pm_buf = [0u8; 20];
                let pm_b = itoa_buf(pm, &mut pm_buf);
                w!(w, b"from :")?;
                w!(w, pm_b)?;
                w!(w, b"\n")?;
            }

            w!(w, b"M 100644 inline ")?;
            w!(w, file_path)?;
            w!(w, b"\n")?;
            let mut blen_buf = [0u8; 20];
            let blen_b = itoa_buf(body.len() as u64, &mut blen_buf);
            w!(w, b"data ")?;
            w!(w, blen_b)?;
            w!(w, b"\n")?;
            w!(w, &body)?;

            prev_mark = Some(mark);
        }

        total_commits += commits_per_batch;
    }

    w.flush().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    let inner = w.into_inner().map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
    std::mem::forget(inner);

    Ok(total_commits)
}
