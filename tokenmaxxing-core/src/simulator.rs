use crate::engine::{EstimationMode, TokenEngine};
use crate::repository::SmartRepositoryMode;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulationResult {
    pub total_tokens: usize,
    pub context_utilization: f64,
    pub projected_savings: usize,
    pub overflow_risk: String,
    pub recommendations: Vec<String>,
}

pub struct ContextSimulator;

impl Default for ContextSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn simulate_repository(&self, path: &Path) -> SimulationResult {
        let engine = TokenEngine::new(EstimationMode::Hybrid);
        let repo_count = engine.count_repository(path);

        let smart = SmartRepositoryMode::new();
        let waste = smart.analyze_waste(path);

        let total_tokens = repo_count.tokens;
        // Claude 3 context window is 200,000 tokens
        let context_window = 200_000.0;
        let context_utilization = (total_tokens as f64 / context_window) * 100.0;

        let overflow_risk = if context_utilization > 90.0 {
            "High".to_string()
        } else if context_utilization > 50.0 {
            "Medium".to_string()
        } else {
            "Low".to_string()
        };

        // Waste optimization savings estimation:
        // We'll estimate that we save roughly 80% of waste bytes in tokens
        let waste_tokens = (waste.total_waste_bytes as f64 / 3.5) as usize;
        let projected_savings = waste_tokens.min(total_tokens);

        let mut recommendations = Vec::new();
        if !waste.duplicates.is_empty() {
            recommendations.push(format!(
                "Remove {} duplicate file pairs.",
                waste.duplicates.len()
            ));
        }
        if !waste.vendor_directories.is_empty() {
            recommendations.push(
                "Add vendor/dependency folders to your .gitignore/exclude parameters.".to_string(),
            );
        }
        if !waste.oversized_files.is_empty() {
            recommendations.push(format!(
                "Deduplicate or split {} oversized files (>50KB).",
                waste.oversized_files.len()
            ));
        }

        if recommendations.is_empty() {
            recommendations.push("Repository is already highly optimized. Great job!".to_string());
        }

        SimulationResult {
            total_tokens,
            context_utilization: context_utilization.min(100.0),
            projected_savings,
            overflow_risk,
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation() {
        let sim = ContextSimulator::new();
        let res = sim.simulate_repository(Path::new("."));
        assert!(res.context_utilization >= 0.0);
    }
}
