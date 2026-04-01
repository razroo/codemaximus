import os
import random
from concurrent.futures import Executor, ThreadPoolExecutor
from typing import Optional

from codemaximus.config import GenerationConfig
from codemaximus.generators import get_generators
from codemaximus.generators.base import GeneratedFile


def _generate_one(args: tuple[float, str, int]) -> GeneratedFile:
    """Worker for parallel batch generation (must be top-level for multiprocessing)."""
    sanity, lang, file_index = args
    random.seed(file_index)
    generators = get_generators(lang)
    gen = random.choice(generators)
    return gen.generate(sanity, file_index)


def _write_one(item: tuple[GeneratedFile, str]) -> int:
    f, output_dir = item
    path = os.path.join(output_dir, f.filename)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(f.content)
    return f.line_count


def generate(config: GenerationConfig) -> list[GeneratedFile]:
    generators = get_generators(config.lang)
    files: list[GeneratedFile] = []
    total_lines = 0
    file_index = 0

    while total_lines < config.lines:
        gen = random.choice(generators)
        result = gen.generate(config.sanity, file_index)
        files.append(result)
        total_lines += result.line_count
        file_index += 1

    return files


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
        return list(executor.map(_generate_one, args))

    generators = get_generators(config.lang)
    files: list[GeneratedFile] = []
    for i in range(batch_size):
        gen = random.choice(generators)
        files.append(gen.generate(config.sanity, start_index + i))
    return files


def write_files(
    files: list[GeneratedFile],
    output_dir: str,
    io_executor: Optional[Executor] = None,
) -> int:
    if not files:
        return 0
    if len(files) == 1:
        f = files[0]
        path = os.path.join(output_dir, f.filename)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(f.content)
        return f.line_count

    items = [(f, output_dir) for f in files]
    if io_executor is not None:
        return sum(io_executor.map(_write_one, items))

    workers = min(16, len(files))
    with ThreadPoolExecutor(max_workers=workers) as pool:
        totals = list(pool.map(_write_one, items))
    return sum(totals)
