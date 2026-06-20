use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use tokenmaxxing_core::benchmark::BenchmarkFramework;
use tokenmaxxing_core::compressor::ContextCompressor;
use tokenmaxxing_core::context::ContextMaxxingEngine;
use tokenmaxxing_core::dashboard::PerformanceDashboard;
use tokenmaxxing_core::engine::{EstimationMode, TokenEngine};
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::pipeline::AutoPipeline;
use tokenmaxxing_core::repository::SmartRepositoryMode;
use tokenmaxxing_core::rewrite::{PromptRewriter, RewriteMode};
use tokenmaxxing_core::simulator::ContextSimulator;

#[derive(Parser)]
#[command(name = "tokenmaxxing")]
#[command(about = "Maximize Claude Code efficiency by reducing unnecessary token consumption", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Count tokens in a file or directory
    Count {
        path: PathBuf,
        #[arg(short, long)]
        accurate: bool,
    },
    /// Optimize a prompt file
    Optimize { path: PathBuf },
    /// Compress context in a file or directory
    Compress { path: PathBuf },
    /// Analyze a repository for bloat and duplicates
    Analyze { path: PathBuf },
    /// Show token stats for a directory
    Stats { path: PathBuf },
    /// Generate a full optimization report
    Report { path: PathBuf },
    /// Run benchmarks against tokenizers
    Benchmark { path: PathBuf },
    /// Evaluate context score, quality, and optimization opportunities
    Context { path: PathBuf },
    /// Rank semantic chunks in a context file by importance
    Rank { path: PathBuf },
    /// Detect repository waste, build artifacts, cache files, and duplicates
    Waste { path: PathBuf },
    /// Simulate context window utilization, overflow risk, and savings
    Simulate { path: PathBuf },
    /// Run the auto-optimization pipeline and print recommendations
    Auto { path: PathBuf },
    /// Rewrite prompts to preserve meaning while stripping token bloat
    Rewrite {
        path: PathBuf,
        #[arg(short, long)]
        conservative: bool,
        #[arg(short, long)]
        aggressive: bool,
    },
    /// Generate architecture and component summaries
    Summarize { path: PathBuf },
    /// Generate a persistent repository memory summary
    Memory { path: PathBuf },
    /// Generate an HTML, Markdown, or JSON Performance Dashboard
    Dashboard {
        path: PathBuf,
        #[arg(short, long)]
        json: bool,
        #[arg(long)]
        html: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Count { path, accurate } => {
            let mode = if *accurate {
                EstimationMode::Accurate
            } else {
                EstimationMode::Fast
            };
            let engine = TokenEngine::new(mode);

            if path.is_file() {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        let count = engine.count_text(&content);
                        println!("Tokens: {}", count.tokens);
                        println!("Characters: {}", count.chars);
                    }
                    Err(e) => eprintln!("Error: Failed to read file '{}': {}", path.display(), e),
                }
            } else {
                let count = engine.count_repository(path);
                println!("Total Repository Tokens: {}", count.tokens);
            }
        }
        Commands::Optimize { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let optimizer = PromptOptimizer::new();
                let (optimized, report) = optimizer.optimize_prompt(&content);

                println!("Optimized Output:\n{}", optimized);
                println!("\n--- Report ---");
                println!("Original chars: {}", report.original_chars);
                println!("Optimized chars: {}", report.optimized_chars);
                println!("Reduction: {:.2}%", report.reduction_percentage);
            }
            Err(e) => eprintln!("Error: Failed to read file '{}': {}", path.display(), e),
        },
        Commands::Compress { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let compressor = ContextCompressor::new();
                let compressed = compressor.compress(&content);
                println!("Compressed Content:\n{}", compressed);
            }
            Err(e) => eprintln!("Error: Failed to read file '{}': {}", path.display(), e),
        },
        Commands::Analyze { path } => {
            println!(
                "Running Repository Intelligence Suite on {}...",
                path.display()
            );
            let smart = SmartRepositoryMode::new();
            let waste = smart.analyze_waste(path);

            println!("Efficiency Score: {:.2}/100", waste.efficiency_score);
            println!("Total Waste Bytes: {} bytes", waste.total_waste_bytes);
            println!("Oversized Files (>50KB): {}", waste.oversized_files.len());
            println!("Suspected Duplicates: {}", waste.duplicates.len());
        }
        Commands::Stats { path } => {
            let engine = TokenEngine::new(EstimationMode::Hybrid);
            let count = engine.count_repository(path);
            println!("Repository Stats for {}", path.display());
            println!("Estimated Tokens (Hybrid Mode): {}", count.tokens);
        }
        Commands::Report { path } => {
            println!(
                "Generating full token optimization report for {}...",
                path.display()
            );
            let pipeline = AutoPipeline::new();
            let report = pipeline.run_pipeline(path);

            let dashboard = PerformanceDashboard::new();
            println!("{}", dashboard.generate_markdown(&report));
        }
        Commands::Benchmark { path } => {
            println!(
                "Benchmarking TokenMaxxing against standard tokenizers on {}...",
                path.display()
            );
            let framework = BenchmarkFramework::new();
            let results = framework.run_benchmarks(path);
            println!("{}", framework.generate_report(&results));

            // Save results to BENCHMARK_RESULTS.md
            let report = framework.generate_report(&results);
            let _ = fs::write("BENCHMARK_RESULTS.md", report);
            println!("Saved results to BENCHMARK_RESULTS.md");
        }
        Commands::Context { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let engine = ContextMaxxingEngine::new();
                let score = engine.score_context(&content);
                let dead = engine.detect_dead_context(&content);

                println!("Context Quality Score: {:.2}/100", score.quality_score);
                println!("Overall Score: {:.2}/100", score.overall_score);
                println!("Noise Ratio: {:.2}", score.noise_ratio);
                println!(
                    "Estimated savings opportunities (dead chunks): {}",
                    dead.len()
                );
            }
            Err(e) => eprintln!("Error reading file: {}", e),
        },
        Commands::Rank { path } => match fs::read_to_string(path) {
            Ok(content) => {
                let engine = ContextMaxxingEngine::new();
                let ranked = engine.rank_chunks(&content);
                println!("Ranked semantic chunks:");
                for chunk in ranked.iter().take(5) {
                    println!(
                        "Rank {}: Score {:.1} (Reason: {}) - Size: {} chars",
                        chunk.rank, chunk.score, chunk.reason, chunk.original_size
                    );
                }
            }
            Err(e) => eprintln!("Error reading file: {}", e),
        },
        Commands::Waste { path } => {
            let smart = SmartRepositoryMode::new();
            let waste = smart.analyze_waste(path);
            println!("--- Repository Waste Report ---");
            println!("Efficiency Score: {:.2}/100", waste.efficiency_score);
            println!("Total Waste Bytes: {} bytes", waste.total_waste_bytes);
            println!("Cache Files: {}", waste.cache_files.len());
            println!("Build Artifacts: {}", waste.build_artifacts.len());
            println!(
                "Vendor/Dependency Files: {}",
                waste.vendor_directories.len()
            );
            println!("Oversized Files (>50KB): {}", waste.oversized_files.len());
            println!("Duplicate Files: {}", waste.duplicates.len());
        }
        Commands::Simulate { path } => {
            let sim = ContextSimulator::new();
            let result = sim.simulate_repository(path);
            println!("--- Context Simulation ---");
            println!("Total Tokens: {}", result.total_tokens);
            println!("Context Utilization: {:.2}%", result.context_utilization);
            println!("Overflow Risk: {}", result.overflow_risk);
            println!("Projected Savings: {} tokens", result.projected_savings);
            println!("\nRecommendations:");
            for rec in &result.recommendations {
                println!("- {}", rec);
            }
        }
        Commands::Auto { path } => {
            let pipeline = AutoPipeline::new();
            let report = pipeline.run_pipeline(path);
            println!("Auto optimization pipeline successfully completed.");
            println!("Initial Tokens: {}", report.initial_tokens);
            println!("Optimization Score: {:.2}/100", report.optimization_score);
            println!(
                "Recommendations: {}",
                report.simulation.recommendations.len()
            );
        }
        Commands::Rewrite {
            path,
            conservative,
            aggressive,
        } => match fs::read_to_string(path) {
            Ok(content) => {
                let mode = if *aggressive {
                    RewriteMode::Aggressive
                } else if *conservative {
                    RewriteMode::Conservative
                } else {
                    RewriteMode::Balanced
                };
                let rewriter = PromptRewriter::new();
                let (rewritten, report) = rewriter.rewrite(&content, mode);
                println!("Rewritten Output:\n{}", rewritten);
                println!("\n--- Rewrite Report ---");
                println!("Original Size: {} chars", report.original_size);
                println!("Optimized Size: {} chars", report.optimized_size);
                println!("Reduction: {:.2}%", report.reduction_percentage);
            }
            Err(e) => eprintln!("Error reading file: {}", e),
        },
        Commands::Summarize { path } => {
            let smart = SmartRepositoryMode::new();
            let memory = smart.generate_memory(path);
            println!("Architecture Summary: {}", memory.architecture_summary);
            println!("Components Detected: {:?}", memory.component_summary);
        }
        Commands::Memory { path } => {
            let smart = SmartRepositoryMode::new();
            let memory = smart.generate_memory(path);
            println!("--- Repository Memory System ---");
            println!("Summary: {}", memory.architecture_summary);
            println!(
                "Active Directory Counts: {}",
                memory.directory_summary.len()
            );
            println!("Parsed Dependencies: {:?}", memory.dependency_summary);
        }
        Commands::Dashboard { path, json, html } => {
            let pipeline = AutoPipeline::new();
            let report = pipeline.run_pipeline(path);
            let dashboard = PerformanceDashboard::new();

            if *json {
                println!("{}", dashboard.generate_json(&report));
            } else if *html {
                println!("{}", dashboard.generate_html(&report));
            } else {
                println!("{}", dashboard.generate_markdown(&report));
            }
        }
    }
}
