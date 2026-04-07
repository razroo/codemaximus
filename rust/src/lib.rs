//! PyO3 extension for `codemaximus` (line counting + fast slop emitters).

mod generic_emit;
mod go_emit;
mod hyperdrive;
mod java_emit;
mod javascript_emit;
mod namegen;
mod python_emit;

use memchr::memchr_iter;
use pyo3::prelude::*;

/// Count newline (`\\n`) bytes — matches `str.count("\\n")` for common text.
#[pyfunction]
#[pyo3(text_signature = "(text)")]
fn line_count(text: &str) -> usize {
    memchr_iter(b'\n', text.as_bytes()).count()
}

/// Generate one generic-Python file body in Rust (same shapes as ``GenericGenerator``).
#[pyfunction]
#[pyo3(text_signature = "(sanity, file_index)")]
fn generate_generic_py(sanity: f64, file_index: i64) -> PyResult<(String, usize, String)> {
    Ok(generic_emit::generate_generic(sanity, file_index))
}

#[pyfunction]
#[pyo3(text_signature = "(sanity, file_index)")]
fn generate_java_py(sanity: f64, file_index: i64) -> PyResult<(String, usize, String)> {
    Ok(java_emit::generate_java(sanity, file_index))
}

#[pyfunction]
#[pyo3(text_signature = "(sanity, file_index)")]
fn generate_enterprise_python_py(sanity: f64, file_index: i64) -> PyResult<(String, usize, String)> {
    Ok(python_emit::generate_enterprise_python(sanity, file_index))
}

/// Returns ``(content, line_count, module_name, ext)`` where ``ext`` is ``.js`` or ``.ts``.
#[pyfunction]
#[pyo3(text_signature = "(sanity, file_index)")]
fn generate_javascript_py(
    sanity: f64,
    file_index: i64,
) -> PyResult<(String, usize, String, String)> {
    Ok(javascript_emit::generate_javascript(sanity, file_index))
}

#[pyfunction]
#[pyo3(text_signature = "(sanity, file_index)")]
fn generate_go_py(sanity: f64, file_index: i64) -> PyResult<(String, usize, String)> {
    Ok(go_emit::generate_go(sanity, file_index))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(line_count, m)?)?;
    m.add_function(wrap_pyfunction!(generate_generic_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_java_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_enterprise_python_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_javascript_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_go_py, m)?)?;
    m.add_function(wrap_pyfunction!(hyperdrive::build_fast_import_stream, m)?)?;
    m.add_function(wrap_pyfunction!(hyperdrive::stream_fast_import_to_fd, m)?)?;
    m.add_function(wrap_pyfunction!(hyperdrive::stream_multi_batch_to_fd, m)?)?;
    Ok(())
}
