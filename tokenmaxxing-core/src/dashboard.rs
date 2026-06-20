use crate::pipeline::PipelineReport;

pub struct PerformanceDashboard;

impl Default for PerformanceDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_markdown(&self, report: &PipelineReport) -> String {
        let mut md = String::new();
        md.push_str("# TokenMaxxing Performance Dashboard\n\n");
        md.push_str(&format!(
            "**Target Location**: `{}`\n",
            report.target_path.display()
        ));
        md.push_str(&format!(
            "**Repository Efficiency Score**: `{:.2}/100`\n",
            report.waste_analysis.efficiency_score
        ));
        md.push_str(&format!(
            "**Aggregate Optimization Score**: `{:.2}/100`\n\n",
            report.optimization_score
        ));

        md.push_str("## Token Assessment\n");
        md.push_str(&format!(
            "- **Initial Count Estimate**: {} tokens\n",
            report.initial_tokens
        ));
        md.push_str(&format!(
            "- **Context Window Utilization**: {:.2}%\n",
            report.simulation.context_utilization
        ));
        md.push_str(&format!(
            "- **Overflow Risk**: `{}`\n",
            report.simulation.overflow_risk
        ));
        md.push_str(&format!(
            "- **Projected Savings**: {} tokens\n\n",
            report.simulation.projected_savings
        ));

        md.push_str("## Waste Analysis Summary\n");
        md.push_str(&format!(
            "- **Total Waste Bytes Found**: {} bytes\n",
            report.waste_analysis.total_waste_bytes
        ));
        md.push_str(&format!(
            "- **Build Artifact Files**: {}\n",
            report.waste_analysis.build_artifacts.len()
        ));
        md.push_str(&format!(
            "- **Cache Files**: {}\n",
            report.waste_analysis.cache_files.len()
        ));
        md.push_str(&format!(
            "- **Vendor Files**: {}\n",
            report.waste_analysis.vendor_directories.len()
        ));
        md.push_str(&format!(
            "- **Duplicate File Pairs**: {}\n\n",
            report.waste_analysis.duplicates.len()
        ));

        md.push_str("## Top Recommendations\n");
        for rec in &report.simulation.recommendations {
            md.push_str(&format!("- [ ] {}\n", rec));
        }

        md
    }

    pub fn generate_html(&self, report: &PipelineReport) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<title>TokenMaxxing Dashboard</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: sans-serif; margin: 40px; background-color: #f7f9fa; color: #333; }\n");
        html.push_str("h1 { color: #1a73e8; }\n");
        html.push_str(".card { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 20px; }\n");
        html.push_str(".score { font-size: 24px; font-weight: bold; color: #202124; }\n");
        html.push_str("</style>\n</head>\n<body>\n");
        html.push_str("<h1>TokenMaxxing Performance Dashboard</h1>\n");
        html.push_str(&format!(
            "<div class='card'><h2>Target Path: {}</h2></div>\n",
            report.target_path.display()
        ));

        html.push_str("<div class='card'>\n");
        html.push_str(&format!(
            "<h3>Optimization Score: <span class='score'>{:.2}/100</span></h3>\n",
            report.optimization_score
        ));
        html.push_str(&format!(
            "<p>Efficiency: {:.2}/100</p>\n",
            report.waste_analysis.efficiency_score
        ));
        html.push_str("</div>\n");

        html.push_str("<div class='card'>\n<h3>Token Statistics</h3>\n<ul>\n");
        html.push_str(&format!(
            "<li>Initial Token Estimate: {}</li>\n",
            report.initial_tokens
        ));
        html.push_str(&format!(
            "<li>Context Utilization: {:.2}%</li>\n",
            report.simulation.context_utilization
        ));
        html.push_str(&format!(
            "<li>Overflow Risk: {}</li>\n",
            report.simulation.overflow_risk
        ));
        html.push_str(&format!(
            "<li>Projected Savings: {} tokens</li>\n",
            report.simulation.projected_savings
        ));
        html.push_str("</ul>\n</div>\n");

        html.push_str("</body>\n</html>");
        html
    }

    pub fn generate_json(&self, report: &PipelineReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}
