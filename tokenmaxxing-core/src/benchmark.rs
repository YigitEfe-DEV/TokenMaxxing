use crate::compressor::ContextCompressor;
use crate::engine::{EstimationMode, TokenEngine};
use crate::optimizer::PromptOptimizer;
use crate::repository::SmartRepositoryMode;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResults {
    pub counting_speed_ms: u128,
    pub compression_ratio: f64,
    pub optimization_ratio: f64,
    pub repository_analysis_speed_ms: u128,
}

pub struct BenchmarkFramework;

impl Default for BenchmarkFramework {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkFramework {
    pub fn new() -> Self {
        Self
    }

    pub fn run_benchmarks(&self, path: &Path) -> BenchmarkResults {
        // Benchmark counting speed
        let start = Instant::now();
        let engine = TokenEngine::new(EstimationMode::Hybrid);
        let _ = engine.count_repository(path);
        let counting_speed_ms = start.elapsed().as_millis();

        // Sample text for compression / optimization benchmarks
        let sample = "This is a sample prompt. Please carefully analyze the codebase and let me know about it.\n\n\
                      Duplicate block of text containing license info under the MIT copyright terms.\n\n\
                      Duplicate block of text containing license info under the MIT copyright terms.";

        let compressor = ContextCompressor::new();
        let compressed = compressor.compress(sample);
        let compression_ratio = compressed.len() as f64 / sample.len() as f64;

        let optimizer = PromptOptimizer::new();
        let (optimized, _) = optimizer.optimize_prompt(sample);
        let optimization_ratio = optimized.len() as f64 / sample.len() as f64;

        // Benchmark repository analysis speed
        let start_intel = Instant::now();
        let smart = SmartRepositoryMode::new();
        let _ = smart.analyze_waste(path);
        let repository_analysis_speed_ms = start_intel.elapsed().as_millis();

        BenchmarkResults {
            counting_speed_ms,
            compression_ratio,
            optimization_ratio,
            repository_analysis_speed_ms,
        }
    }

    pub fn generate_report(&self, results: &BenchmarkResults) -> String {
        let mut report = String::new();
        report.push_str("# TokenMaxxing Benchmark Results\n\n");
        report.push_str("| Metric | Value |\n");
        report.push_str("| :--- | :--- |\n");
        report.push_str(&format!(
            "| Counting Speed | {} ms |\n",
            results.counting_speed_ms
        ));
        report.push_str(&format!(
            "| Compression Ratio | {:.2}% (lower is better) |\n",
            results.compression_ratio * 100.0
        ));
        report.push_str(&format!(
            "| Optimization Ratio | {:.2}% (lower is better) |\n",
            results.optimization_ratio * 100.0
        ));
        report.push_str(&format!(
            "| Repository Analysis Speed | {} ms |\n",
            results.repository_analysis_speed_ms
        ));
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmarks() {
        let framework = BenchmarkFramework::new();
        let results = framework.run_benchmarks(Path::new("."));
        assert!(results.compression_ratio > 0.0);
    }
}
