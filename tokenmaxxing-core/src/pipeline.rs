use crate::engine::{EstimationMode, TokenEngine};
use crate::repository::{SmartRepositoryMode, WasteReport};
use crate::simulator::{ContextSimulator, SimulationResult};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineReport {
    pub target_path: PathBuf,
    pub initial_tokens: usize,
    pub waste_analysis: WasteReport,
    pub simulation: SimulationResult,
    pub optimization_score: f64,
}

pub struct AutoPipeline;

impl Default for AutoPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoPipeline {
    pub fn new() -> Self {
        Self
    }

    pub fn run_pipeline(&self, path: &Path) -> PipelineReport {
        // 1. Scan / Estimate tokens
        let engine = TokenEngine::new(EstimationMode::Hybrid);
        let repo_count = engine.count_repository(path);

        // 2. Analyze Waste / Duplicates / Junk
        let smart = SmartRepositoryMode::new();
        let waste_analysis = smart.analyze_waste(path);

        // 3. Simulate optimizer improvements
        let simulator = ContextSimulator::new();
        let simulation = simulator.simulate_repository(path);

        // 4. Calculate an aggregate Optimization Score
        let raw_score = waste_analysis.efficiency_score;
        let scale_factor = if simulation.context_utilization > 80.0 {
            0.8 // penalty for very high context usage
        } else {
            1.0
        };
        let optimization_score = (raw_score * scale_factor).clamp(0.0, 100.0);

        PipelineReport {
            target_path: path.to_path_buf(),
            initial_tokens: repo_count.tokens,
            waste_analysis,
            simulation,
            optimization_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline() {
        let pipeline = AutoPipeline::new();
        let report = pipeline.run_pipeline(Path::new("."));
        assert!(report.optimization_score <= 100.0);
    }
}
