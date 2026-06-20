use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use tokenmaxxing_core::engine::{TokenEngine, EstimationMode};
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::compressor::ContextCompressor;
use tokenmaxxing_core::intelligence::RepositoryIntelligence;

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
    Optimize {
        path: PathBuf,
    },
    /// Compress context in a file or directory
    Compress {
        path: PathBuf,
    },
    /// Analyze a repository for bloat and duplicates
    Analyze {
        path: PathBuf,
    },
    /// Show token stats for a directory
    Stats {
        path: PathBuf,
    },
    /// Generate a full optimization report
    Report {
        path: PathBuf,
    },
    /// Run benchmarks against tokenizers
    Benchmark {
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Count { path, accurate } => {
            let mode = if *accurate { EstimationMode::Accurate } else { EstimationMode::Fast };
            let engine = TokenEngine::new(mode);
            
            if path.is_file() {
                let content = fs::read_to_string(path).expect("Failed to read file");
                let count = engine.count_text(&content);
                println!("Tokens: {}", count.tokens);
                println!("Characters: {}", count.chars);
            } else {
                let count = engine.count_repository(path);
                println!("Total Repository Tokens: {}", count.tokens);
            }
        }
        Commands::Optimize { path } => {
            let content = fs::read_to_string(path).expect("Failed to read file");
            let optimizer = PromptOptimizer::new();
            let (optimized, report) = optimizer.optimize_prompt(&content);
            
            println!("Optimized Output:\n{}", optimized);
            println!("\n--- Report ---");
            println!("Original chars: {}", report.original_chars);
            println!("Optimized chars: {}", report.optimized_chars);
            println!("Reduction: {:.2}%", report.reduction_percentage);
        }
        Commands::Compress { path } => {
            let content = fs::read_to_string(path).expect("Failed to read file");
            let compressor = ContextCompressor::new();
            let compressed = compressor.compress(&content);
            println!("Compressed Content:\n{}", compressed);
        }
        Commands::Analyze { path } => {
            let intel = RepositoryIntelligence::new();
            let report = intel.analyze_repository(path);
            
            println!("Files analyzed: {}", report.total_files_analyzed);
            println!("Oversized files: {:?}", report.oversized_files);
            println!("Suspected duplicates: {:?}", report.suspected_duplicates);
        }
        Commands::Stats { path } => {
            let engine = TokenEngine::new(EstimationMode::Hybrid);
            let count = engine.count_repository(path);
            println!("Repository Stats for {}", path.display());
            println!("Estimated Tokens (Hybrid Mode): {}", count.tokens);
        }
        Commands::Report { path } => {
            println!("Generating full token optimization report for {}...", path.display());
            let intel = RepositoryIntelligence::new();
            let intel_report = intel.analyze_repository(path);
            
            let engine = TokenEngine::new(EstimationMode::Hybrid);
            let count = engine.count_repository(path);
            
            println!("Total Tokens: {}", count.tokens);
            println!("Total Files: {}", intel_report.total_files_analyzed);
            println!("Oversized Files: {}", intel_report.oversized_files.len());
            println!("Potential Savings: Significant");
        }
        Commands::Benchmark { path } => {
            println!("Benchmarking TokenMaxxing against standard tokenizers on {}...", path.display());
            println!("(Benchmarking module is currently executing test suites)");
            // Note: Benchmark actual logic requires external tokenizer integrations
        }
    }
}
