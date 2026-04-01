import argparse
import importlib.metadata
import sys
import time

from codemaximus.config import GenerationConfig


def _package_version() -> str:
    try:
        return importlib.metadata.version("codemaximus")
    except importlib.metadata.PackageNotFoundError:
        return "0.0.0-dev"


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "hyperdrive":
        sys.argv.pop(1)
        from codemaximus.hyperdrive import main as hyperdrive_main

        return hyperdrive_main()

    parser = argparse.ArgumentParser(
        prog="codemaximus",
        description=(
            "Synthesize multi-language slop; optional turbo loop (generate, git add, commit)."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "examples:\n"
            "  codemaximus --lines 10000 --sanity 100\n"
            "  codemaximus --lines 50000 --sanity 0 --lang java\n"
            "  codemaximus --lines 100000 --turbo --sanity 50\n"
            "  codemaximus --lines 1000 --enterprise\n"
            "  codemaximus --turbo --forever --push-every 50 --batch-size 30\n"
            "  codemaximus --turbo --forever --branch main --push-every 100\n"
            "  codemaximus --turbo --lines 20000 --workers 4 --batch-size 20\n"
            "  codemaximus --lines 10000 --dry-run\n"
            "  codemaximus --turbo --lines 50000 --dry-run\n"
            "  codemaximus hyperdrive --commits 10000 --branch main --push\n"
            "  codemaximus hyperdrive --commits 10000 --batches 3 --push\n"
        ),
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {_package_version()}",
    )

    parser.add_argument(
        "--lines", type=int, default=10000,
        help="Target number of lines to generate (default: 10000)",
    )
    parser.add_argument(
        "--sanity", type=int, default=50, choices=range(0, 101),
        metavar="0-100",
        help="Chaos level. 100=corporate cringe, 0=full unhinged (default: 50)",
    )
    parser.add_argument(
        "--lang", type=str, default="all",
        choices=["all", "java", "python", "js", "javascript", "go", "generic"],
        help="Target language(s) for generated slop (default: all)",
    )
    parser.add_argument(
        "--output", type=str, default="./output",
        help="Output directory (default: ./output)",
    )
    parser.add_argument(
        "--turbo", action="store_true",
        help="Turbo mode: generate -> git add -> commit -> repeat",
    )
    parser.add_argument(
        "--enterprise", action="store_true",
        help="Enterprise mode: 10x line multiplier because enterprise",
    )
    parser.add_argument(
        "--batch-size", type=int, default=15,
        help="Files per commit in turbo mode (default: 15, higher = more lines/commit)",
    )
    parser.add_argument(
        "--push-every", type=int, default=0,
        help="Auto-push every N commits (default: 0 = manual push)",
    )
    parser.add_argument(
        "--forever", action="store_true",
        help="Run indefinitely until interrupted (ignores --lines)",
    )
    parser.add_argument(
        "--branch", type=str, default="",
        help="Use this branch instead of creating slop/session-* (useful for CI)",
    )
    parser.add_argument(
        "--workers", type=int, default=0, metavar="N",
        help="Parallel generator processes in turbo (0 = auto, default: 0)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help=(
            "Do not write files or run git. Estimates lines/files from in-memory generation "
            "(turbo: no branch setup or commits; normal: no output on disk)."
        ),
    )

    args = parser.parse_args()

    config = GenerationConfig(
        lines=args.lines,
        sanity=args.sanity / 100.0,
        lang=args.lang,
        output_dir=args.output,
        turbo=args.turbo,
        enterprise=args.enterprise,
        batch_size=args.batch_size,
        push_every=args.push_every,
        forever=args.forever,
        branch=args.branch,
        workers=args.workers,
        dry_run=args.dry_run,
    )

    if config.turbo:
        from codemaximus.turbo import run_turbo
        run_turbo(config)
    else:
        from codemaximus.generator import generate_to_directory

        tag = " (dry-run)" if config.dry_run else ""
        print(f"Generating {config.lines:,} lines of pure slop{tag}...")
        print(f"Sanity: {config.sanity:.0%} | Lang: {config.lang} | Output: {config.output_dir}")
        print()

        t0 = time.perf_counter()
        total, nfiles, samples = generate_to_directory(
            config,
            config.output_dir,
            dry_run=config.dry_run,
            sample_limit=5,
        )
        elapsed = time.perf_counter() - t0
        rate = total / elapsed if elapsed > 0 else 0.0
        if config.dry_run:
            print(f"Would write {nfiles:,} files ({total:,} lines, on target).")
            print(f"Generation wall time: {elapsed:.2f}s (~{rate:,.0f} lines/s).")
            if samples:
                print(f"Sample paths ({len(samples)}/{nfiles}):")
                for path in samples:
                    print(f"  {path}")
        else:
            print(f"Done! Generated {total:,} lines across {nfiles:,} files.")
            print(f"Wall time: {elapsed:.2f}s (~{rate:,.0f} lines/s).")
            print(f"Output: {config.output_dir}/")


if __name__ == "__main__":
    main()
