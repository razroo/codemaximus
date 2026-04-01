import os
import subprocess
import sys
import time
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor

from codemaximus.config import GenerationConfig
from codemaximus.generator import generate_batch, trim_batch_to_line_budget, write_files
from codemaximus.generators.base import GeneratedFile
from codemaximus.naming import commit_message
from codemaximus.stats import Stats

# Avoid hitting OS argv limits when --batch-size is huge.
_GIT_ADD_PATH_CHUNK = 256


def _git_env() -> dict[str, str]:
    """Non-interactive Git (avoids terminal prompts in CI / headless runs)."""
    return {**os.environ, "GIT_TERMINAL_PROMPT": "0"}


def _stage_batch_files(output_dir: str, files: list[GeneratedFile]) -> None:
    """Stage only paths written this batch (avoids re-scanning the whole output tree)."""
    cwd = os.getcwd()
    rels = [
        os.path.relpath(os.path.join(output_dir, f.filename), cwd)
        for f in files
    ]
    for i in range(0, len(rels), _GIT_ADD_PATH_CHUNK):
        chunk = rels[i : i + _GIT_ADD_PATH_CHUNK]
        subprocess.run(
            ["git", "add", "--", *chunk],
            capture_output=True,
            env=_git_env(),
        )


def _turbo_workers(config: GenerationConfig) -> int:
    if config.workers > 0:
        return max(1, config.workers)
    return max(1, min(8, os.cpu_count() or 4))


def git_push(silent=True):
    result = subprocess.run(
        ["git", "push"],
        capture_output=True,
        text=True,
        env=_git_env(),
    )
    if not silent and result.returncode != 0:
        print(f"\nPush failed: {result.stderr.strip()}")
    return result.returncode == 0


def _remote_policy_notice(kind: str) -> None:
    print(
        f"Notice ({kind}): pushing or automating many commits against a remote can violate "
        "host terms or org policy. Use only where explicitly allowed.\n"
    )


def run_turbo(config: GenerationConfig):
    output_dir = os.path.abspath(config.output_dir)
    if not config.dry_run:
        os.makedirs(output_dir, exist_ok=True)

    branch_name = ""
    if not config.dry_run:
        # check if we're in a git repo
        result = subprocess.run(
            ["git", "rev-parse", "--is-inside-work-tree"],
            capture_output=True,
            text=True,
            env=_git_env(),
        )
        if result.returncode != 0:
            print("Error: not inside a git repository. Initialize one first.")
            print("  git init && git add -A && git commit -m 'init'")
            sys.exit(1)

        # branch setup
        if config.branch:
            # use specified branch (for CI / --forever restarts)
            current = subprocess.run(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"],
                capture_output=True,
                text=True,
                env=_git_env(),
            ).stdout.strip()
            if current != config.branch:
                subprocess.run(
                    ["git", "checkout", "-B", config.branch],
                    capture_output=True,
                    env=_git_env(),
                )
            branch_name = config.branch
        else:
            branch_name = f"slop/session-{int(time.time())}"
            subprocess.run(
                ["git", "checkout", "-b", branch_name],
                capture_output=True,
                env=_git_env(),
            )

        if config.push_every > 0:
            _remote_policy_notice("turbo --push-every")

    mode = "FOREVER" if config.forever else "TURBO"
    if config.dry_run:
        mode += "-DRYRUN"
    if config.dry_run:
        print(f"[{mode}] No git or disk writes; generation only.")
        print(f"[{mode}] Hypothetical output dir: {output_dir}")
    else:
        print(f"[{mode}] Branch: {branch_name}")
    print(f"[{mode}] Target: {'∞' if config.forever else f'{config.lines:,}'} lines | Sanity: {config.sanity:.0%} | Lang: {config.lang}")
    tw = _turbo_workers(config)
    print(f"[{mode}] Batch: {config.batch_size} files/commit | Push every: {config.push_every or 'never'}")
    print(f"[{mode}] Workers: {tw} ({'auto' if config.workers == 0 else 'fixed'})")
    print(f"[{mode}] Output: {output_dir}")
    print()

    stats = Stats()
    file_index = 0
    commits_since_push = 0

    def _one_iteration(proc_pool, io_pool):
        nonlocal file_index, commits_since_push
        remaining = None if config.forever else (config.lines - stats.total_lines)
        if remaining is not None and remaining <= 0:
            return

        files = generate_batch(
            config, config.batch_size, file_index, executor=proc_pool
        )
        file_index += config.batch_size
        if remaining is not None:
            files = trim_batch_to_line_budget(files, remaining)
        if not files:
            return

        if config.dry_run:
            lines = sum(f.line_count for f in files)
        else:
            lines = write_files(
                files,
                output_dir,
                io_executor=io_pool if len(files) > 1 else None,
            )

        if config.dry_run:
            stats.add(lines=lines, files=len(files), commits=0)
        else:
            _stage_batch_files(output_dir, files)
            msg = commit_message(config.sanity, stats.total_commits + 1)
            subprocess.run(
                [
                    "git",
                    "commit",
                    "-m",
                    msg,
                    "--allow-empty",
                    "--no-verify",
                ],
                capture_output=True,
                env=_git_env(),
            )

            stats.add(lines=lines, files=len(files), commits=1)
            commits_since_push += 1

            if config.push_every and commits_since_push >= config.push_every:
                git_push()
                commits_since_push = 0

        sys.stdout.write(stats.display(mode))
        sys.stdout.flush()

    io_workers = min(16, max(2, config.batch_size))

    def _loop(proc_pool, io_pool):
        while config.forever or stats.total_lines < config.lines:
            _one_iteration(proc_pool, io_pool)

    try:
        if config.batch_size <= 1:
            if tw <= 1:
                _loop(None, None)
            else:
                with ProcessPoolExecutor(max_workers=tw) as proc_pool:
                    _loop(proc_pool, None)
        else:
            with ThreadPoolExecutor(max_workers=io_workers) as io_pool:
                if tw <= 1:
                    _loop(None, io_pool)
                else:
                    with ProcessPoolExecutor(max_workers=tw) as proc_pool:
                        _loop(proc_pool, io_pool)

    except KeyboardInterrupt:
        print(f"\n\n{mode} mode interrupted!")

    # final push if we have unpushed commits
    if not config.dry_run and config.push_every and commits_since_push > 0:
        print("\nPushing remaining commits...")
        git_push(silent=False)

    print(
        stats.summary(
            note="(dry-run: no commits or files written)" if config.dry_run else None
        )
    )
