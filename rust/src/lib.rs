//! PyO3 extension scaffold for `codemaximus`.
//! Exposes small helpers that can grow over time (e.g. faster line counting, buffers).

use pyo3::prelude::*;

/// Count newline (`\\n`) bytes — matches `str.count("\\n")` for common text.
#[pyfunction]
#[pyo3(text_signature = "(text)")]
fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(line_count, m)?)?;
    Ok(())
}
