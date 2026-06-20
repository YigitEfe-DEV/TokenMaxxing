#![deny(clippy::all)]

use napi_derive::napi;
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::compressor::ContextCompressor;
use tokenmaxxing_core::engine::{TokenEngine, EstimationMode};

#[napi]
pub fn optimize_prompt(text: String) -> String {
    let optimizer = PromptOptimizer::new();
    let (optimized, _) = optimizer.optimize_prompt(&text);
    optimized
}

#[napi]
pub fn compress_context(text: String) -> String {
    let compressor = ContextCompressor::new();
    compressor.compress(&text)
}

#[napi]
pub fn count_tokens_fast(text: String) -> u32 {
    let engine = TokenEngine::new(EstimationMode::Fast);
    engine.count_text(&text).tokens as u32
}
