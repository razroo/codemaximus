"""
Hyperdrive: append thousands of commits via `git fast-import`, then one `git push`.

Why this exists: `git commit` per commit pays fork/exec and index work each time.
`fast-import` streams commit objects in one process — the practical way to create
10k+ commits quickly. GitHub sees them after a single push (one packfile upload),
not 10k separate pushes.

v2 — stream building is now done in Rust (when the native extension is available),
and the stream is piped directly into `git fast-import` via fd instead of buffering
the entire payload in Python memory first.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from typing import Optional

from codemaximus.native import (
    native_build_fast_import_stream,
    native_hyperdrive_direct,
    native_stream_fast_import_to_fd,
    native_stream_multi_batch_to_fd,
)
from codemaximus.turbo import _git_env


def _fast_import_env() -> dict[str, str]:
    """Git env tuned for maximum fast-import throughput."""
    env = _git_env()
    # Disable fsync — we don't need crash safety for synthetic commits
    env["GIT_FLUSH"] = "0"
    return env


def _fast_import_cmd() -> list[str]:
    """git fast-import with performance flags."""
    return [
        "git",
        "-c", "core.fsyncMethod=batch",
        "-c", "core.fsync=none",
        "-c", "pack.threads=0",           # auto-detect CPU count for packing
        "-c", "gc.auto=0",                # don't trigger gc mid-import
        "-c", "pack.compression=0",       # skip zlib compression (speed > size)
        "-c", "pack.depth=0",             # no delta chains
        "fast-import",
        "--quiet",
        "--force",
        "--done",                          # stream ends with explicit 'done' marker
    ]


def _check_repo() -> None:
    r = subprocess.run(
        ["git", "rev-parse", "--is-inside-work-tree"],
        capture_output=True,
        text=True,
        env=_git_env(),
    )
    if r.returncode != 0:
        print("Error: not inside a git repository.", file=sys.stderr)
        sys.exit(1)


def _resolve_parent_ref(branch: str) -> Optional[str]:
    """SHA of current branch tip, or None if ref does not exist yet."""
    r = subprocess.run(
        ["git", "rev-parse", "--verify", f"refs/heads/{branch}"],
        capture_output=True,
        text=True,
        env=_git_env(),
    )
    if r.returncode != 0:
        return None
    return r.stdout.strip()


def _build_fast_import_stream_py(
    *,
    branch: str,
    n: int,
    parent_sha: Optional[str],
    batch_tag: int,
    base_ts: int,
) -> bytes:
    """Pure-Python fallback when the Rust extension is not available."""
    chunks: list[bytes] = []
    ref_line = f"commit refs/heads/{branch}\n".encode()
    author_prefix = b"author cm <cm@h> "
    committer_prefix = b"committer cm <cm@h> "
    tz_suffix = b" +0000\n"
    tag_b = str(batch_tag).encode()

    # Shared blob as mark :1
    blob_content = b"h\n"
    chunks.append(b"blob\nmark :1\ndata " + str(len(blob_content)).encode() + b"\n" + blob_content + b"\n")

    for i in range(n):
        mark = i + 2  # marks start at 2 (1 is the blob)
        msg_b = tag_b + b"." + str(i + 1).encode()
        ts_b = str(base_ts + i).encode()

        chunks.append(ref_line)
        chunks.append(b"mark :" + str(mark).encode() + b"\n")
        chunks.append(author_prefix + ts_b + tz_suffix)
        chunks.append(committer_prefix + ts_b + tz_suffix)
        chunks.append(b"data " + str(len(msg_b)).encode() + b"\n" + msg_b + b"\n")

        if i == 0:
            if parent_sha is not None:
                chunks.append(f"from {parent_sha}\n".encode())
        else:
            chunks.append(b"from :" + str(mark - 1).encode() + b"\n")

        # Reference shared blob by mark instead of inline
        chunks.append(b"M 100644 :1 s\n")

    return b"".join(chunks)


def _run_fast_import_streamed(
    *,
    branch: str,
    n: int,
    parent_sha: Optional[str],
    batch_tag: int,
    base_ts: int,
) -> tuple[float, float]:
    """
    Pipe the Rust-built stream directly into git fast-import via fd.
    Returns (build_time, import_time) — with streaming these overlap,
    so build_time is the total wall time and import_time is 0.
    """
    proc = subprocess.Popen(
        _fast_import_cmd(),
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=_fast_import_env(),
    )
    assert proc.stdin is not None

    t0 = time.perf_counter()
    fd = proc.stdin.fileno()
    native_stream_fast_import_to_fd(
        fd, branch, n, parent_sha, batch_tag, base_ts,
    )
    # Send 'done' marker for --done flag
    proc.stdin.write(b"done\n")
    proc.stdin.close()
    err = proc.stderr.read() if proc.stderr else b""
    code = proc.wait()
    t1 = time.perf_counter()

    if code != 0:
        print(err.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"git fast-import failed with exit code {code}", file=sys.stderr)
        sys.exit(1)

    return (t1 - t0, 0.0)


def _run_fast_import_buffered(
    *,
    branch: str,
    n: int,
    parent_sha: Optional[str],
    batch_tag: int,
    base_ts: int,
) -> tuple[float, float]:
    """
    Build stream (Rust or Python), then feed to git fast-import.
    Returns (build_time, import_time).
    """
    t0 = time.perf_counter()
    if native_build_fast_import_stream is not None:
        stream = native_build_fast_import_stream(
            branch, n, parent_sha, batch_tag, base_ts,
        )
    else:
        stream = _build_fast_import_stream_py(
            branch=branch,
            n=n,
            parent_sha=parent_sha,
            batch_tag=batch_tag,
            base_ts=base_ts,
        )
    t1 = time.perf_counter()

    proc = subprocess.Popen(
        _fast_import_cmd(),
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=_fast_import_env(),
    )
    assert proc.stdin is not None
    proc.stdin.write(stream)
    # Send 'done' marker for --done flag
    proc.stdin.write(b"done\n")
    proc.stdin.close()
    err = proc.stderr.read() if proc.stderr else b""
    code = proc.wait()
    t2 = time.perf_counter()

    if code != 0:
        print(err.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"git fast-import failed with exit code {code}", file=sys.stderr)
        sys.exit(1)

    return (t1 - t0, t2 - t1)


def run_batch(
    *,
    branch: str,
    n: int,
    parent_sha: Optional[str],
    batch_tag: int,
    base_ts: int,
) -> tuple[float, float]:
    """Run one batch — streaming if Rust fd support is available, buffered otherwise."""
    if native_stream_fast_import_to_fd is not None:
        return _run_fast_import_streamed(
            branch=branch, n=n, parent_sha=parent_sha,
            batch_tag=batch_tag, base_ts=base_ts,
        )
    return _run_fast_import_buffered(
        branch=branch, n=n, parent_sha=parent_sha,
        batch_tag=batch_tag, base_ts=base_ts,
    )


def _run_multi_batch_streamed(
    *,
    branch: str,
    batches: int,
    commits_per_batch: int,
    parent_sha: Optional[str],
    base_batch_tag: int,
    base_ts: int,
) -> float:
    """
    Stream ALL batches through a single git fast-import process.
    Produces exactly 1 pack file. Returns total wall time.
    """
    proc = subprocess.Popen(
        _fast_import_cmd(),
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=_fast_import_env(),
    )
    assert proc.stdin is not None

    t0 = time.perf_counter()
    fd = proc.stdin.fileno()
    native_stream_multi_batch_to_fd(
        fd, branch, batches, commits_per_batch,
        parent_sha, base_batch_tag, base_ts,
    )
    proc.stdin.write(b"done\n")
    proc.stdin.close()
    err = proc.stderr.read() if proc.stderr else b""
    code = proc.wait()
    t1 = time.perf_counter()

    if code != 0:
        print(err.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"git fast-import failed with exit code {code}", file=sys.stderr)
        sys.exit(1)

    return t1 - t0


def run_maintenance() -> None:
    """Run git multi-pack-index and commit-graph to speed up future operations."""
    print("[hyperdrive] running maintenance (multi-pack-index + commit-graph)...")
    t0 = time.perf_counter()
    subprocess.run(
        ["git", "multi-pack-index", "write"],
        capture_output=True, env=_git_env(),
    )
    subprocess.run(
        ["git", "commit-graph", "write", "--reachable", "--changed-paths"],
        capture_output=True, env=_git_env(),
    )
    t1 = time.perf_counter()
    print(f"[hyperdrive] maintenance done: {(t1 - t0) * 1000:.1f} ms")


def git_push_branch(remote: str, branch: str) -> bool:
    r = subprocess.run(
        ["git", "push", "-u", remote, f"{branch}:{branch}"],
        capture_output=True,
        text=True,
        env=_git_env(),
    )
    if r.returncode != 0:
        print(r.stderr.strip() or r.stdout.strip(), file=sys.stderr)
        return False
    return True


def main() -> None:
    import argparse

    p = argparse.ArgumentParser(
        prog="codemaximus hyperdrive",
        description=(
            "Create many commits in one git fast-import stream, then optionally push once. "
            "Append to an existing branch to add another batch."
        ),
    )
    p.add_argument(
        "--commits",
        type=int,
        default=10_000,
        help="Number of commits in this batch (default: 10000)",
    )
    p.add_argument(
        "--branch",
        type=str,
        default="main",
        help="Branch to advance (default: main)",
    )
    p.add_argument(
        "--batch-tag",
        type=int,
        default=0,
        help="Monotonic id embedded in messages (default: auto from time)",
    )
    p.add_argument(
        "--push",
        action="store_true",
        help="git push once after fast-import (single round-trip to GitHub)",
    )
    p.add_argument(
        "--remote",
        type=str,
        default="origin",
        help="Remote for --push (default: origin)",
    )
    p.add_argument(
        "--batches",
        type=int,
        default=1,
        help=(
            "Run this many back-to-back fast-import batches of --commits each "
            "(each batch appends to the same branch; default: 1)"
        ),
    )
    p.add_argument(
        "--maintain",
        action="store_true",
        help="Run git multi-pack-index + commit-graph after import (speeds up push)",
    )
    args = p.parse_args()

    if args.commits < 1:
        print("--commits must be >= 1", file=sys.stderr)
        sys.exit(1)
    if args.batches < 1:
        print("--batches must be >= 1", file=sys.stderr)
        sys.exit(1)

    _check_repo()

    can_direct = native_hyperdrive_direct is not None
    can_multi = native_stream_multi_batch_to_fd is not None
    backend = (
        "direct-pack" if can_direct else
        "rust-multi-stream" if can_multi else
        "rust-streaming" if native_stream_fast_import_to_fd is not None else
        "rust-buffered" if native_build_fast_import_stream is not None else
        "python"
    )
    print(f"[hyperdrive] backend: {backend}")

    base_tag = args.batch_tag or int(time.time()) % 1_000_000_000
    total_commits = args.commits * args.batches

    if can_direct:
        # Direct packfile writer — no git fast-import subprocess at all
        repo_path = os.getcwd()
        parent = _resolve_parent_ref(args.branch)
        base_ts = int(time.time())

        print(
            f"[hyperdrive] direct pack: {args.batches} batches x {args.commits:,} = "
            f"{total_commits:,} commits (no subprocess)"
        )
        print(
            f"[hyperdrive] branch={args.branch} "
            f"parent={'(new root)' if parent is None else parent[:12] + '…'} "
            f"base_tag={base_tag}"
        )

        written, total_wall = native_hyperdrive_direct(
            repo_path, args.branch, args.batches, args.commits,
            parent, base_tag, base_ts,
        )
        total_commits = written

    elif can_multi and args.batches > 1:
        # Single fast-import process for ALL batches → 1 pack file
        parent = _resolve_parent_ref(args.branch)
        base_ts = int(time.time())

        print(
            f"[hyperdrive] streaming {args.batches} batches x {args.commits:,} = "
            f"{total_commits:,} commits through single fast-import process"
        )
        print(
            f"[hyperdrive] branch={args.branch} "
            f"parent={'(new root)' if parent is None else parent[:12] + '…'} "
            f"base_tag={base_tag}"
        )

        total_wall = _run_multi_batch_streamed(
            branch=args.branch,
            batches=args.batches,
            commits_per_batch=args.commits,
            parent_sha=parent,
            base_batch_tag=base_tag,
            base_ts=base_ts,
        )
    else:
        # Fallback: one fast-import per batch
        total_wall = 0.0
        total_commits = 0

        for batch_idx in range(args.batches):
            parent = _resolve_parent_ref(args.branch)
            batch_tag = base_tag + batch_idx
            base_ts = int(time.time())

            print(
                f"[hyperdrive] batch {batch_idx + 1}/{args.batches} "
                f"branch={args.branch} commits={args.commits:,} "
                f"parent={'(new root)' if parent is None else parent[:12] + '…'} "
                f"batch_tag={batch_tag}"
            )

            build_t, import_t = run_batch(
                branch=args.branch,
                n=args.commits,
                parent_sha=parent,
                batch_tag=batch_tag,
                base_ts=base_ts,
            )

            wall = build_t + import_t
            total_wall += wall
            total_commits += args.commits

            if import_t > 0:
                print(
                    f"[hyperdrive] stream build: {build_t * 1000:.1f} ms | "
                    f"fast-import: {import_t * 1000:.1f} ms"
                )
            else:
                print(
                    f"[hyperdrive] streamed batch: {build_t * 1000:.1f} ms"
                )

    rate = total_commits / total_wall if total_wall > 0 else 0
    print(
        f"[hyperdrive] done: {args.batches} batches | {total_commits:,} commits | "
        f"wall: {total_wall * 1000:.1f} ms | {rate:,.0f} commits/s"
    )

    if args.maintain:
        run_maintenance()

    if args.push:
        print(
            "Notice (hyperdrive --push): ensure host and org policy allow bulk commits "
            "before pushing.",
            file=sys.stderr,
        )
        t3 = time.perf_counter()
        ok = git_push_branch(args.remote, args.branch)
        t4 = time.perf_counter()
        print(f"[hyperdrive] git push: {(t4 - t3) * 1000:.1f} ms (success={ok})")
        if not ok:
            sys.exit(1)
    else:
        print("[hyperdrive] Skipped push. Run: git push -u", args.remote, args.branch)


if __name__ == "__main__":
    main()
