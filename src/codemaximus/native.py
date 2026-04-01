"""Optional Rust-accelerated helpers; fall back to pure Python when `_native` is missing."""

try:
    from codemaximus._native import line_count as native_line_count
except ImportError:
    native_line_count = None


def line_count(text: str) -> int:
    if native_line_count is not None:
        return native_line_count(text)
    return text.count("\n")
