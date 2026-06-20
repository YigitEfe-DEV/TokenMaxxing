use pyo3::prelude::*;
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::compressor::ContextCompressor;
use tokenmaxxing_core::engine::{TokenEngine, EstimationMode};

#[pyfunction]
fn optimize_prompt(text: &str) -> PyResult<String> {
    let optimizer = PromptOptimizer::new();
    let (optimized, _) = optimizer.optimize_prompt(text);
    Ok(optimized)
}

#[pyfunction]
fn compress_context(text: &str) -> PyResult<String> {
    let compressor = ContextCompressor::new();
    Ok(compressor.compress(text))
}

#[pyfunction]
fn count_tokens_fast(text: &str) -> PyResult<usize> {
    let engine = TokenEngine::new(EstimationMode::Fast);
    Ok(engine.count_text(text).tokens)
}

#[pymodule]
fn tokenmaxxing(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(optimize_prompt, m)?)?;
    m.add_function(wrap_pyfunction!(compress_context, m)?)?;
    m.add_function(wrap_pyfunction!(count_tokens_fast, m)?)?;
    Ok(())
}
