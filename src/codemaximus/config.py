from dataclasses import dataclass


@dataclass
class GenerationConfig:
    lines: int = 10000
    sanity: float = 0.5  # 0.0 = full chaos, 1.0 = corporate cringe
    lang: str = "all"
    output_dir: str = "./output"
    turbo: bool = False
    enterprise: bool = False
    batch_size: int = 15
    push_every: int = 0  # auto-push every N commits (0 = never)
    forever: bool = False  # run indefinitely
    branch: str = ""  # use existing branch instead of creating new one
    workers: int = 0  # 0 = auto (min(8, CPU count)); else parallel generator processes
    dry_run: bool = False  # no writes / no git (turbo simulates generation throughput only)

    def __post_init__(self):
        self.sanity = max(0.0, min(1.0, self.sanity))
        if self.enterprise:
            self.lines *= 10
        if self.workers < 0:
            raise ValueError("workers must be >= 0")
        if self.workers > 64:
            self.workers = 64
