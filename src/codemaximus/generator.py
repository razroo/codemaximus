import os
import random
from collections.abc import Iterator
from concurrent.futures import Executor, ThreadPoolExecutor
from typing import Optional

from codemaximus.config import GenerationConfig
from codemaximus.generators import get_generators
from codemaximus.generators.base import GeneratedFile
from codemaximus.native import line_count as count_newlines

# Larger buffer reduces syscall overhead for multi-MB slop files (text mode).
_WRITE_BUFFER_BYTES = 1 << 20

# Forkserver workers call ``get_generators`` once per language per process.
_GEN_CACHE: dict[str, list] = {}


def truncate_content_to_newlines(content: str, n: int) -> str:
    """Keep a prefix of `content` that contains exactly `n` newline characters (or all content if shorter)."""
    if n <= 0:
        return ""
    pos = -1
    for _ in range(n):
        pos = content.find("\n", pos + 1)
        if pos == -1:
            return content
    return content[: pos + 1]


def truncate_generated_to_line_budget(result: GeneratedFile, max_newlines: int) -> GeneratedFile:
    """Match project line accounting: line_count == count of '\\n' in content."""
    if max_newlines <= 0:
        return GeneratedFile(filename=result.filename, content="", line_count=0)
    if result.line_count <= max_newlines:
        return result
    new_content = truncate_content_to_newlines(result.content, max_newlines)
    return GeneratedFile(
        filename=result.filename,
        content=new_content,
        line_count=count_newlines(new_content),
    )


def trim_batch_to_line_budget(
    files: list[GeneratedFile], budget: int | None
) -> list[GeneratedFile]:
    """Drop or truncate from the end until sum(line_count) <= budget. budget None = no trim."""
    if budget is None:
        return files
    out = list(files)
    total = sum(f.line_count for f in out)
    while total > budget and out:
        over = total - budget
        last = out[-1]
        if last.line_count > over:
            keep = last.line_count - over
            out[-1] = truncate_generated_to_line_budget(last, keep)
            total = budget
        else:
            total -= last.line_count
            out.pop()
    return out


def _iter_files_until_target(config: GenerationConfig) -> Iterator[GeneratedFile]:
    total_lines = 0
    file_index = 0
    generators = get_generators(config.lang)
    while total_lines < config.lines:
        need = config.lines - total_lines
        gen = random.choice(generators)
        result = gen.generate(config.sanity, file_index)
        file_index += 1
        if result.line_count > need:
            result = truncate_generated_to_line_budget(result, need)
        if result.line_count == 0:
            continue
        total_lines += result.line_count
        yield result


def generate(config: GenerationConfig) -> list[GeneratedFile]:
    """Return all bodies in memory; for large ``--lines`` use ``generate_to_directory`` instead."""
    return list(_iter_files_until_target(config))


def generate_to_directory(
    config: GenerationConfig,
    output_dir: str,
    *,
    dry_run: bool = False,
    sample_limit: int = 5,
) -> tuple[int, int, list[str]]:
    """
    Stream one file at a time to disk (or dry-run) so large --lines runs do not retain
    all file bodies in memory. Returns (total_lines, file_count, sample_paths).
    """
    total_lines = 0
    nfiles = 0
    samples: list[str] = []
    mkdir_cache: set[str] = set()
    for result in _iter_files_until_target(config):
        if len(samples) < sample_limit:
            samples.append(result.filename)
        if not dry_run:
            write_single(result, output_dir, mkdir_cache=mkdir_cache)
        total_lines += result.line_count
        nfiles += 1
    return total_lines, nfiles, samples


def _generate_one(args: tuple[float, str, int]) -> GeneratedFile:
    """Worker for parallel batch generation (must be top-level for multiprocessing)."""
    sanity, lang, file_index = args
    random.seed(file_index)
    generators = _GEN_CACHE.get(lang)
    if generators is None:
        generators = get_generators(lang)
        _GEN_CACHE[lang] = generators
    gen = random.choice(generators)
    return gen.generate(sanity, file_index)


def generate_batch(
    config: GenerationConfig,
    batch_size: int,
    start_index: int,
    executor: Optional[Executor] = None,
) -> list[GeneratedFile]:
    args = [
        (config.sanity, config.lang, start_index + i)
        for i in range(batch_size)
    ]
    if executor is not None:
        # Larger chunks amortize IPC for big batches (ProcessPoolExecutor).
        n = len(args)
        cpus = os.cpu_count() or 4
        chunksize = max(1, min(32, n // max(1, cpus)))
        return list(executor.map(_generate_one, args, chunksize=chunksize))

    generators = get_generators(config.lang)
    files: list[GeneratedFile] = []
    for i in range(batch_size):
        gen = random.choice(generators)
        files.append(gen.generate(config.sanity, start_index + i))
    return files


def _ensure_parent_dirs(output_dir: str, files: list[GeneratedFile]) -> None:
    dirs: set[str] = set()
    for f in files:
        d = os.path.dirname(os.path.join(output_dir, f.filename))
        if d:
            dirs.add(d)
    for d in dirs:
        os.makedirs(d, exist_ok=True)


def write_single(
    file: GeneratedFile,
    output_dir: str,
    *,
    mkdir_cache: set[str] | None = None,
) -> int:
    path = os.path.join(output_dir, file.filename)
    d = os.path.dirname(path)
    if d:
        if mkdir_cache is not None:
            if d not in mkdir_cache:
                os.makedirs(d, exist_ok=True)
                mkdir_cache.add(d)
        else:
            os.makedirs(d, exist_ok=True)
    with open(path, "w", encoding="utf-8", buffering=_WRITE_BUFFER_BYTES) as fh:
        fh.write(file.content)
    return file.line_count


def _write_one(item: tuple[GeneratedFile, str]) -> int:
    f, output_dir = item
    path = os.path.join(output_dir, f.filename)
    with open(path, "w", encoding="utf-8", buffering=_WRITE_BUFFER_BYTES) as fh:
        fh.write(f.content)
    return f.line_count


def write_files(
    files: list[GeneratedFile],
    output_dir: str,
    io_executor: Optional[Executor] = None,
) -> int:
    if not files:
        return 0
    if len(files) == 1:
        return write_single(files[0], output_dir, mkdir_cache=None)

    _ensure_parent_dirs(output_dir, files)
    items = [(f, output_dir) for f in files]
    if io_executor is not None:
        return sum(io_executor.map(_write_one, items))

    workers = min(16, len(files))
    with ThreadPoolExecutor(max_workers=workers) as pool:
        totals = list(pool.map(_write_one, items))
    return sum(totals)
