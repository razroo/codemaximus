"""Type stubs for the optional Rust extension ``codemaximus._native``."""

def line_count(text: str) -> int:
    """Count ``\\n`` characters (same idea as ``text.count('\\n')``)."""

def generate_generic_py(sanity: float, file_index: int) -> tuple[str, int, str]:
    """Return ``(content, newline_count, class_name)`` for one generic slop file."""

def generate_java_py(sanity: float, file_index: int) -> tuple[str, int, str]:
    """Return ``(content, newline_count, class_name)`` for one Java file."""

def generate_enterprise_python_py(sanity: float, file_index: int) -> tuple[str, int, str]:
    """Return ``(content, newline_count, class_name)`` for one enterprise Python file."""

def generate_javascript_py(sanity: float, file_index: int) -> tuple[str, int, str, str]:
    """Return ``(content, newline_count, module_name, ext)`` where ``ext`` is ``.js`` or ``.ts``."""

def generate_go_py(sanity: float, file_index: int) -> tuple[str, int, str]:
    """Return ``(content, newline_count, struct_name)`` for one Go file."""
