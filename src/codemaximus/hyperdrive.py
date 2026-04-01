"""
Hyperdrive: append thousands of commits via `git fast-import`, then one `git push`.

Why this exists: `git commit` per commit pays fork/exec and index work each time.
`fast-import` streams commit objects in one process — the practical way to create
10k+ commits quickly. GitHub sees them after a single push (one packfile upload),
not 10k separate pushes.

Reality check: "1 second on GitHub" depends on network and GitHub processing the
packfile; local `fast-import` is usually sub-second to a few seconds for 10k commits.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from typing import Optional

from codemaximus.turbo import _git_env


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


def _build_fast_import_stream(
    *,
    branch: str,
    n: int,
    parent_sha: Optional[str],
    batch_tag: int,
) -> bytes:
    """Linear chain of n commits touching one file; advances refs/heads/<branch>."""
    chunks: list[bytes] = []
    name = "Codemaximus Hyperdrive"
    email = "hyperdrive@codemaximus.local"
    path = "slop/hyperdrive_signal.txt"
    base_ts = int(time.time())

    for i in range(n):
        mark = i + 1
        msg = f"hyperdrive batch {batch_tag} commit {i + 1}/{n}"
        msg_b = msg.encode("utf-8")
        body = f"{batch_tag}:{i + 1}:{base_ts + i}\n".encode("utf-8")

        chunks.append(f"commit refs/heads/{branch}\n".encode())
        chunks.append(f"mark :{mark}\n".encode())
        chunks.append(
            f"author {name} <{email}> {base_ts + i} +0000\n".encode()
        )
        chunks.append(
            f"committer {name} <{email}> {base_ts + i} +0000\n".encode()
        )
        chunks.append(f"data {len(msg_b)}\n".encode() + msg_b + b"\n")

        if i == 0:
            if parent_sha is not None:
                chunks.append(f"from {parent_sha}\n".encode())
        else:
            chunks.append(f"from :{mark - 1}\n".encode())

        chunks.append(f"M 100644 inline {path}\n".encode())
        chunks.append(f"data {len(body)}\n".encode() + body)

    return b"".join(chunks)


def run_fast_import(stream: bytes) -> None:
    proc = subprocess.Popen(
        ["git", "fast-import", "--quiet", "--force"],
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=_git_env(),
    )
    assert proc.stdin is not None
    proc.stdin.write(stream)
    proc.stdin.close()
    err = proc.stderr.read() if proc.stderr else b""
    code = proc.wait()
    if code != 0:
        print(err.decode("utf-8", errors="replace"), file=sys.stderr)
        print(f"git fast-import failed with exit code {code}", file=sys.stderr)
        sys.exit(1)


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
    args = p.parse_args()

    if args.commits < 1:
        print("--commits must be >= 1", file=sys.stderr)
        sys.exit(1)
    if args.batches < 1:
        print("--batches must be >= 1", file=sys.stderr)
        sys.exit(1)

    _check_repo()

    base_tag = args.batch_tag or int(time.time()) % 1_000_000_000

    total_build = 0.0
    total_import = 0.0
    total_commits = 0

    for batch_idx in range(args.batches):
        parent = _resolve_parent_ref(args.branch)
        batch_tag = base_tag + batch_idx

        print(
            f"[hyperdrive] batch {batch_idx + 1}/{args.batches} "
            f"branch={args.branch} commits={args.commits:,} "
            f"parent={'(new root)' if parent is None else parent[:12] + '…'} "
            f"batch_tag={batch_tag}"
        )

        t0 = time.perf_counter()
        stream = _build_fast_import_stream(
            branch=args.branch,
            n=args.commits,
            parent_sha=parent,
            batch_tag=batch_tag,
        )
        t1 = time.perf_counter()
        run_fast_import(stream)
        t2 = time.perf_counter()

        total_build += t1 - t0
        total_import += t2 - t1
        total_commits += args.commits

        print(
            f"[hyperdrive] stream build: {(t1 - t0) * 1000:.1f} ms | "
            f"fast-import: {(t2 - t1) * 1000:.1f} ms"
        )

    print(
        f"[hyperdrive] batches done: {args.batches} | commits written: {total_commits:,} | "
        f"build sum: {total_build * 1000:.1f} ms | fast-import sum: {total_import * 1000:.1f} ms"
    )

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
