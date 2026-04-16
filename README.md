<div align="center">

# codemaximus

**What it is:** a small command-line tool that **synthesizes large amounts of plausible-looking code** in several languages (Java, Python, JS/TS, Go, generic). You control how “corporate” vs chaotic the names and comments look.

**What it is for:** satire, demos, and experiments around **repository size and commit cadence**—for example pairing with **turbo mode** (`generate → git add → commit` in a loop) so you can see how fast your machine and Git can churn through synthetic files.

**What it is not:** production software, a substitute for real tests, or something you should use to misrepresent real work. Treat Git hosts’ terms of service and your org’s policies as the source of truth.

### [`codemaxxed`](https://github.com/jshchnz/codemaxxed)

_a live repo being continuously codemaxxed_

<!-- LINES_BADGE -->
<img src="https://img.shields.io/badge/lines%20of%20code-335%2C015%2C880-brightgreen?style=for-the-badge" alt="Lines of Code">
<!-- /LINES_BADGE -->

<!-- FILES_BADGE -->
<img src="https://img.shields.io/badge/files-1%2C178%2C040-blue?style=for-the-badge" alt="Files">
<!-- /FILES_BADGE -->
<!-- COMMITS_BADGE -->
<img src="https://img.shields.io/badge/commits-45%2C206-orange?style=for-the-badge" alt="Commits">
<!-- /COMMITS_BADGE -->

</div>

---

## Why Python

The tool is almost entirely **string synthesis** plus **filesystem and Git subprocesses**. Python keeps the generators easy to read and change. Throughput in turbo mode is dominated by **Git and disk**, not the interpreter; the CLI uses **parallel batch generation** (worker processes) and **scoped `git add`** so each loop does less work than `git add -A` on a huge tree.

If you needed absolute maximum single-threaded generation speed, a rewrite in **Rust** or **Go** could help the CPU-bound part—but you would still pay Git’s cost unless you also changed how often you commit or how large each commit is.

There is also an **optional PyO3 extension** (`codemaximus._native`) built with **maturin**, currently exposing a small `line_count` helper as a starting point for faster primitives.

---

## Installation

You need a **Rust toolchain** (`rustup` + stable `cargo`) so `pip` can compile the native module via **maturin**.

```bash
git clone https://github.com/jshchnz/codemaximus.git
cd codemaximus
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install .
```

Editable install (for working on Python and Rust):

```bash
pip install -e .
# or, if you use maturin directly (requires an active venv):
maturin develop --manifest-path rust/Cargo.toml
```

Build a wheel without installing into the env:

```bash
maturin build --manifest-path rust/Cargo.toml
pip install rust/target/wheels/codemaximus-*.whl
```

After install, `from codemaximus import line_count` prefers the Rust implementation when present; `native_line_count` is the raw extension function or `None` if the module was not built (the shim still falls back to `str.count`).

## Quick Start

```bash
# Generate 10,000 lines of corporate-flavored slop
codemaximus --lines 10000 --sanity 100

# Generate 10,000 lines of chaotic slop
codemaximus --lines 10000 --sanity 0

# TURBO: generate, git add only new paths, commit, repeat (parallel batches)
codemaximus --lines 100000 --turbo --sanity 50

# Cap parallel generator workers (default: auto, min(8, CPUs))
codemaximus --turbo --lines 50000 --workers 4

# Enterprise mode (10x line multiplier)
codemaximus --lines 10000 --enterprise

# One language
codemaximus --lines 5000 --lang java --sanity 80
```

### Hyperdrive (many commits, one push)

Normal **`git commit`** in a loop is slow (process and index work per commit). **Hyperdrive** uses **`git fast-import`** to stream a linear chain of commits into the object database in one shot, then **`git push` once** so GitHub receives a single packfile with all of them.

- **Local speed:** tens of thousands of commits often finish in **a few seconds** on a laptop (disk-bound).
- **GitHub:** commits **appear after one push**, not one HTTP round-trip per commit. The push itself is usually **not** “one second” for a large packfile; treat **one fast local import** plus **one push** as the practical pattern.
- **Another 10,000:** run hyperdrive again (it appends), or use **`--batches N`** for multiple waves before **`--push`**.

```bash
codemaximus hyperdrive --commits 10000 --branch main --push
codemaximus hyperdrive --commits 10000 --batches 3 --branch main --push
```

Use only where policy allows. Automated or abusive patterns can violate host **terms of service**.

## Features

### `--sanity` (0–100)

| Sanity | Style | Example |
|--------|-------|---------|
| 100 | Corporate | `AbstractSingletonProxyFactoryBeanManagerImpl` |
| 50 | Mixed | `EnhancedYeetOrchestrator` |
| 0 | Chaos | `xX_Destroyer_Xx_SkibidiHandler_Ligma` |

### `--turbo`

Runs a loop: generate a batch → **`git add` only the files written in that batch** (chunked so huge batches do not overflow the shell argv) → `git commit` → repeat. With **two or more `--workers`**, uses a **reused `ProcessPoolExecutor`** for generation; with **`--workers 1`** or **auto resolving to one process**, generation runs in-process (no pool overhead). With **`--batch-size` ≥ 2**, disk writes use a **reused thread pool** for the whole run (no new pool every commit).

### `--workers` (turbo)

Parallel generator processes: **`0`** (default) means auto — `min(8, CPU count)` — **`1`** forces in-process generation. Values are capped at **64**.

### `--enterprise`

Multiplies the line target by 10.

### Languages

Java, Python, JavaScript/TypeScript, Go, and a generic text style.

## Performance tips

- Prefer a **fast disk** and a repo where the output directory is not scanned by slow hooks; commits use **`--no-verify`** to skip client-side hooks that would run every commit.
- **Larger `--batch-size`** → more lines per commit and fewer Git round-trips per line (until memory or `git add` cost bites).
- Tune **`--workers`** if CPU-bound generation is the bottleneck; use **`--workers 1`** to avoid multiprocessing when debugging or on very small batches.
- **`--push-every N`** spreads network cost; `0` means push only at the end (turbo still pushes leftovers if configured).

## Testimonials

> "I went from 0 to 10,000 commits in a single afternoon. My manager promoted me on the spot."
> — Anonymous 10x Developer

> "Our codebase is now 4 million lines. We've never been more enterprise."
> — CTO, Fortune 500

> "I don't understand any of it but the graphs go up"
> — PM

## Contributing

PRs welcome. If your PR removes lines of code, expect pushback—we only go up here.

## FAQ

**Is this useful?**  
As a serious engineering tool, no. As a toy or benchmark of your machine and Git, maybe.

**Should I use this at work?**  
Only if policy allows and you are not misrepresenting work.

**Does the generated code compile?**  
Sometimes partially; do not rely on it.

## License

MIT SSL (Special Slop License).
