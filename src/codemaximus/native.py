"""Optional Rust-accelerated helpers; fall back to pure Python when `_native` is missing."""

try:
    from codemaximus._native import line_count as native_line_count
    from codemaximus._native import generate_enterprise_python_py as native_generate_enterprise_python
    from codemaximus._native import generate_generic_py as native_generate_generic
    from codemaximus._native import generate_go_py as native_generate_go
    from codemaximus._native import generate_java_py as native_generate_java
    from codemaximus._native import generate_javascript_py as native_generate_javascript
    from codemaximus._native import build_fast_import_stream as native_build_fast_import_stream
    from codemaximus._native import stream_fast_import_to_fd as native_stream_fast_import_to_fd
    from codemaximus._native import stream_multi_batch_to_fd as native_stream_multi_batch_to_fd
except ImportError:
    native_line_count = None
    native_generate_generic = None
    native_generate_java = None
    native_generate_enterprise_python = None
    native_generate_javascript = None
    native_generate_go = None
    native_build_fast_import_stream = None
    native_stream_fast_import_to_fd = None
    native_stream_multi_batch_to_fd = None


def line_count(text: str) -> int:
    if native_line_count is not None:
        return native_line_count(text)
    return text.count("\n")
