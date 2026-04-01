"""Optional Rust-accelerated helpers; fall back to pure Python when `_native` is missing."""

try:
    from codemaximus._native import line_count as native_line_count
    from codemaximus._native import generate_enterprise_python_py as native_generate_enterprise_python
    from codemaximus._native import generate_generic_py as native_generate_generic
    from codemaximus._native import generate_go_py as native_generate_go
    from codemaximus._native import generate_java_py as native_generate_java
    from codemaximus._native import generate_javascript_py as native_generate_javascript
except ImportError:
    native_line_count = None
    native_generate_generic = None
    native_generate_java = None
    native_generate_enterprise_python = None
    native_generate_javascript = None
    native_generate_go = None


def line_count(text: str) -> int:
    if native_line_count is not None:
        return native_line_count(text)
    return text.count("\n")
