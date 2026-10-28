use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokenmaxxing_core::benchmark::BenchmarkFramework;
use tokenmaxxing_core::compressor::ContextCompressor;
use tokenmaxxing_core::context::ContextMaxxingEngine;
use tokenmaxxing_core::cost::{build_optimization_report, CostEngine, Provenance};
use tokenmaxxing_core::dashboard::PerformanceDashboard;
use tokenmaxxing_core::engine::{text_files, EstimationMode, TokenCount, TokenEngine};
use tokenmaxxing_core::optimizer::PromptOptimizer;
use tokenmaxxing_core::pipeline::AutoPipeline;
use tokenmaxxing_core::pricing::PricingProfile;
use tokenmaxxing_core::provider::AnthropicTokenCounter;
use tokenmaxxing_core::repository::SmartRepositoryMode;
use tokenmaxxing_core::rewrite::{PromptRewriter, RewriteMode};
use tokenmaxxing_core::simulator::ContextSimulator;
use tokenmaxxing_core::tokenizer::openai::{self, OpenAiTokenizer};
use tokenmaxxing_core::tokenizer::{LocalTokenizer, TokenCountProvider, TokenizerInfo};

/// Model that is used when the caller does not name an OpenAI model.
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o";

/// How the tokens of a path are counted.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CountMode {
    /// Heuristic estimate, no tokenizer is involved.
    Estimate,
    /// Exact count with a local tokenizer.
    Local,
    /// Exact count from the provider API.
    Api,
    /// Run every available method and compare the results.
    Compare,
}

/// The provider a model belongs to.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderArg {
    /// OpenAI models, counted with the published vocabulary.
    #[value(name = "openai")]
    OpenAi,
    /// Anthropic models, counted through the Anthropic API.
    #[value(name = "anthropic")]
    Anthropic,
}

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
        /// Use the structural approximation instead of the ratio heuristic
        #[arg(short, long)]
        accurate: bool,
        /// How the tokens are counted
        #[arg(short, long, value_enum, default_value_t = CountMode::Estimate)]
        mode: CountMode,
        /// Provider the model belongs to
        #[arg(short, long, value_enum)]
        provider: Option<ProviderArg>,
        /// Model the count refers to
        #[arg(long)]
        model: Option<String>,
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
    /// Report the token and cost effect of optimizing a prompt file
    Cost {
        path: PathBuf,
        /// Pricing profile with dated model rates
        #[arg(long)]
        pricing: PathBuf,
        /// How the tokens are counted
        #[arg(short, long, value_enum, default_value_t = CountMode::Local)]
        mode: CountMode,
        /// Provider the model belongs to
        #[arg(short, long, value_enum)]
        provider: Option<ProviderArg>,
        /// Model the cost refers to
        #[arg(long)]
        model: Option<String>,
    },
    /// List the tokenizers and models that can be counted exactly
    Tokenizers,
}

/// A counting method with the information needed to label its result.
enum Counter {
    Estimate(EstimationMode, TokenEngine),
    Local(TokenEngine, TokenizerInfo),
    Api(Box<dyn TokenCountProvider>),
}

impl Counter {
    fn count(&self, path: &Path) -> Result<TokenCount, String> {
        match self {
            Counter::Estimate(_, engine) => count_with_engine(path, engine),
            Counter::Local(engine, _) => count_with_engine(path, engine),
            Counter::Api(provider) => count_with_provider(path, provider.as_ref()),
        }
    }

    fn count_text(&self, text: &str) -> Result<TokenCount, String> {
        match self {
            Counter::Estimate(_, engine) => Ok(engine.count_text(text)),
            Counter::Local(engine, _) => Ok(engine.count_text(text)),
            Counter::Api(provider) => provider.count(text).map_err(|error| error.to_string()),
        }
    }

    /// Name of the measurement itself.
    fn label(&self) -> String {
        match self {
            Counter::Estimate(EstimationMode::Fast, _) => {
                "character ratio heuristic (3.5 chars per token)".to_string()
            }
            Counter::Estimate(EstimationMode::Accurate, _) => {
                "structural byte-pair encoding emulation".to_string()
            }
            Counter::Estimate(EstimationMode::Hybrid, _) => "hybrid estimator".to_string(),
            Counter::Local(_, info) => format!("{} ({})", info.id, info.version),
            Counter::Api(provider) => provider.info().id,
        }
    }

    fn provenance(&self, model: &str) -> Provenance {
        let mut provenance = Provenance {
            model: Some(model.to_string()),
            ..Provenance::default()
        };

        match self {
            Counter::Estimate(mode, _) => {
                provenance.count_method = match mode {
                    EstimationMode::Accurate => tokenmaxxing_core::CountKind::Approximate,
                    _ => tokenmaxxing_core::CountKind::Estimated,
                };
            }
            Counter::Local(_, info) => {
                provenance.count_method = tokenmaxxing_core::CountKind::ExactLocal;
                provenance.tokenizer = Some(info.id.clone());
                provenance.tokenizer_version = Some(info.version.clone());
                provenance.encoding = Some(info.encoding.clone());
            }
            Counter::Api(provider) => {
                provenance.count_method = tokenmaxxing_core::CountKind::ProviderApi;
                provenance.tokenizer = Some(provider.info().id.clone());
                provenance.encoding = Some(provider.info().endpoint.clone());
            }
        }

        provenance
    }
}

fn estimate_counter(accurate: bool) -> Counter {
    let mode = if accurate {
        EstimationMode::Accurate
    } else {
        EstimationMode::Fast
    };
    Counter::Estimate(mode, TokenEngine::new(mode))
}

fn local_counter(provider: ProviderArg, model: &str) -> Result<Counter, String> {
    match provider {
        ProviderArg::OpenAi => {
            let tokenizer = OpenAiTokenizer::for_model(model).map_err(|error| error.to_string())?;
            let info = tokenizer.info();
            Ok(Counter::Local(
                TokenEngine::with_tokenizer(Arc::new(tokenizer)),
                info,
            ))
        }
        ProviderArg::Anthropic => Err(
            "Anthropic does not publish Claude's tokenizer, so there is no local exact count. \
             Use `--mode api` to ask the Anthropic counting endpoint."
                .to_string(),
        ),
    }
}

fn api_counter(provider: ProviderArg, model: &str) -> Result<Counter, String> {
    match provider {
        ProviderArg::Anthropic => {
            let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
            Ok(Counter::Api(Box::new(AnthropicTokenCounter::new(
                model, api_key,
            ))))
        }
        ProviderArg::OpenAi => Err(
            "OpenAI does not offer a token counting endpoint. Use `--mode local`: \
             the published vocabulary counts exactly."
                .to_string(),
        ),
    }
}

fn default_model(provider: ProviderArg) -> &'static str {
    match provider {
        ProviderArg::OpenAi => DEFAULT_OPENAI_MODEL,
        ProviderArg::Anthropic => AnthropicTokenCounter::DEFAULT_MODEL,
    }
}

fn count_with_engine(path: &Path, engine: &TokenEngine) -> Result<TokenCount, String> {
    if path.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("Failed to read file '{}': {}", path.display(), error))?;
        Ok(engine.count_text(&content))
    } else {
        Ok(engine.count_repository(path))
    }
}

fn count_with_provider(
    path: &Path,
    provider: &dyn TokenCountProvider,
) -> Result<TokenCount, String> {
    if path.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("Failed to read file '{}': {}", path.display(), error))?;
        return provider.count(&content).map_err(|error| error.to_string());
    }

    let mut total = TokenCount::default();
    for file in text_files(path) {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        let count = provider
            .count(&content)
            .map_err(|error| format!("{}: {}", file.display(), error))?;
        total = total + count;
    }
    Ok(total)
}

fn print_count(path: &Path, count: &TokenCount, method: &str) {
    println!("Tokens: {}", count.tokens);
    println!("Count Method: {}", count.kind.label());
    println!("Measurement: {}", method);
    println!("Characters: {}", count.chars);
    println!("Bytes: {}", count.bytes);
    let _ = path;
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Count {
            path,
            accurate,
            mode,
            provider,
            model,
        } => {
            let provider = provider.unwrap_or(ProviderArg::OpenAi);
            let model = model
                .clone()
                .unwrap_or_else(|| default_model(provider).to_string());

            match mode {
                CountMode::Estimate => {
                    let counter = estimate_counter(*accurate);
                    match counter.count(path) {
                        Ok(count) => print_count(path, &count, &counter.label()),
                        Err(error) => eprintln!("Error: {}", error),
                    }
                }
                CountMode::Local => match local_counter(provider, &model) {
                    Ok(counter) => match counter.count(path) {
                        Ok(count) => print_count(path, &count, &counter.label()),
                        Err(error) => eprintln!("Error: {}", error),
                    },
                    Err(error) => eprintln!("Error: {}", error),
                },
                CountMode::Api => match api_counter(provider, &model) {
                    Ok(counter) => match counter.count(path) {
                        Ok(count) => print_count(path, &count, &counter.label()),
                        Err(error) => eprintln!("Error: {}", error),
                    },
                    Err(error) => eprintln!("Error: {}", error),
                },
                CountMode::Compare => {
                    compare_counts(path, provider, &model, *accurate);
                }
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
            println!("Estimated Tokens: {}", count.tokens);
            println!("Count Method: {}", count.kind.label());
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
            println!("Count Method: {}", result.count_method.label());
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
        Commands::Cost {
            path,
            pricing,
            mode,
            provider,
            model,
        } => {
            let provider = provider.unwrap_or(ProviderArg::OpenAi);
            let model = model
                .clone()
                .unwrap_or_else(|| default_model(provider).to_string());

            let profile = match PricingProfile::load_from_path(pricing) {
                Ok(profile) => profile,
                Err(error) => {
                    eprintln!("Error: {}", error);
                    return;
                }
            };

            let counter = match mode {
                CountMode::Estimate => Some(estimate_counter(false)),
                CountMode::Local => match local_counter(provider, &model) {
                    Ok(counter) => Some(counter),
                    Err(error) => {
                        eprintln!("Error: {}", error);
                        return;
                    }
                },
                CountMode::Api => match api_counter(provider, &model) {
                    Ok(counter) => Some(counter),
                    Err(error) => {
                        eprintln!("Error: {}", error);
                        return;
                    }
                },
                CountMode::Compare => {
                    eprintln!("Error: --mode compare does not produce a cost report");
                    return;
                }
            };
            let Some(counter) = counter else {
                return;
            };

            let content = match fs::read_to_string(path) {
                Ok(content) => content,
                Err(error) => {
                    eprintln!("Error: Failed to read file '{}': {}", path.display(), error);
                    return;
                }
            };

            let optimizer = PromptOptimizer::new();
            let (optimized, optimization) = optimizer.optimize_prompt(&content);

            let before = match counter.count_text(&content) {
                Ok(count) => count,
                Err(error) => {
                    eprintln!("Error: {}", error);
                    return;
                }
            };
            let after = match counter.count_text(&optimized) {
                Ok(count) => count,
                Err(error) => {
                    eprintln!("Error: {}", error);
                    return;
                }
            };

            let engine = CostEngine::new(profile);
            let mut provenance = counter.provenance(&model);
            provenance.pricing_effective_date = Some(engine.profile().effective_date.clone());
            provenance.pricing_source = Some(engine.profile().source.clone());

            let report = build_optimization_report(
                &content,
                &optimized,
                before,
                after,
                optimization.removed.clone(),
                provenance,
                Some((&engine, &model)),
            );

            println!("Optimization Report");
            println!("===================");
            println!("Target:        {}", path.display());
            println!("Model:         {}", model);
            println!("Count Method:  {}", report.before.kind.label());
            println!("Measurement:   {}", counter.label());
            println!(
                "Tokenizer:     {}",
                report.provenance.tokenizer.as_deref().unwrap_or("none")
            );
            if let Some(version) = &report.provenance.tokenizer_version {
                println!("Tokenizer Ver: {}", version);
            }
            println!();
            println!(
                "Before:  {} tokens ({} bytes)",
                report.before.tokens, report.before.bytes
            );
            println!(
                "After:   {} tokens ({} bytes)",
                report.after.tokens, report.after.bytes
            );
            println!(
                "Removed: {} tokens ({} bytes)",
                report.removed_tokens(),
                report.removed_bytes
            );

            if !report.removed.is_empty() {
                println!();
                println!("Removed content:");
                for removed in &report.removed {
                    println!(
                        "  - {}: {} bytes, {} lines",
                        removed.category, removed.bytes, removed.lines
                    );
                }
            }

            if let (Some(cost_before), Some(cost_after)) = (&report.cost_before, &report.cost_after)
            {
                println!();
                println!("Estimated cost (input tokens, {}):", cost_before.currency);
                println!("  Before:     {:.6}", cost_before.total);
                println!("  After:      {:.6}", cost_after.total);
                if let (Some(difference), Some(percentage)) = (
                    report.cost_difference(),
                    report.cost_difference_percentage(),
                ) {
                    println!("  Difference: {:.6} ({:+.2}%)", difference, percentage);
                }
                println!(
                    "  Pricing:    effective {}, source {}",
                    cost_before.pricing_effective_date, cost_before.pricing_source
                );
            }

            println!();
            for note in &report.notes {
                println!("Note: {}", note);
            }
        }
        Commands::Tokenizers => {
            println!("Local tokenizers (exact counts, offline):");
            for (encoding, models) in openai::supported_encodings() {
                println!("  {}", encoding);
                for model in models {
                    println!("    - {}", model);
                }
            }
            println!();
            println!("Provider APIs (exact counts for the provider's own model):");
            println!("  anthropic");
            println!(
                "    - endpoint  {}",
                AnthropicTokenCounter::endpoint_for_model()
            );
            println!(
                "    - the tokenizer of Claude is not published, so there is no local \
                 tokenizer for it"
            );
        }
    }
}

/// Runs every available counting method and prints the results side by side.
fn compare_counts(path: &Path, provider: ProviderArg, model: &str, accurate: bool) {
    let mut rows: Vec<(String, Option<TokenCount>, String)> = Vec::new();

    let fast = estimate_counter(false);
    let structural = estimate_counter(true);
    for (label, counter) in [
        ("ratio estimate", fast),
        ("structural approximation", structural),
    ] {
        match counter.count(path) {
            Ok(count) => rows.push((label.to_string(), Some(count), counter.label())),
            Err(error) => rows.push((label.to_string(), None, error)),
        }
    }
    if accurate {
        rows.push((
            "note".to_string(),
            None,
            "--accurate selects the structural approximation for the estimate mode".to_string(),
        ));
    }

    match local_counter(provider, model) {
        Ok(counter) => match counter.count(path) {
            Ok(count) => rows.push((
                format!("local tokenizer ({model})"),
                Some(count),
                counter.label(),
            )),
            Err(error) => rows.push((format!("local tokenizer ({model})"), None, error)),
        },
        Err(error) => rows.push(("local tokenizer".to_string(), None, error)),
    }

    match provider {
        ProviderArg::Anthropic => {
            if std::env::var("ANTHROPIC_API_KEY")
                .map(|key| key.trim().is_empty())
                .unwrap_or(true)
            {
                rows.push((
                    format!("provider API ({model})"),
                    None,
                    "skipped: ANTHROPIC_API_KEY is not set".to_string(),
                ));
            } else {
                match api_counter(provider, model) {
                    Ok(counter) => match counter.count(path) {
                        Ok(count) => rows.push((
                            format!("provider API ({model})"),
                            Some(count),
                            counter.label(),
                        )),
                        Err(error) => rows.push((format!("provider API ({model})"), None, error)),
                    },
                    Err(error) => rows.push((format!("provider API ({model})"), None, error)),
                }
            }
        }
        ProviderArg::OpenAi => rows.push((
            format!("provider API ({model})"),
            None,
            "not available: OpenAI has no token counting endpoint, its vocabulary is \
             counted locally"
                .to_string(),
        )),
    }

    println!("Target: {}", path.display());
    println!();
    println!("{:<34} {:>10}  Count method", "Method", "Tokens");
    println!("{}", "-".repeat(74));
    for (label, count, method) in &rows {
        match count {
            Some(count) => println!(
                "{:<34} {:>10}  {} ({})",
                label,
                count.tokens,
                count.kind.label(),
                method
            ),
            None => println!("{:<34} {:>10}  {}", label, "-", method),
        }
    }

    let exact: Vec<(&str, &TokenCount)> = rows
        .iter()
        .filter_map(|(label, count, _)| count.as_ref().map(|count| (label.as_str(), count)))
        .collect();

    let differences = match exact.first() {
        Some((_, reference)) => exact
            .iter()
            .filter(|(_, count)| count.tokens != reference.tokens)
            .count(),
        None => 0,
    };

    if differences > 0 {
        println!();
        println!("Why the counts differ:");
        println!(
            "  The estimators use a character ratio and a structural emulation of \
             byte-pair encoding."
        );
        println!(
            "  A tokenizer counts the pieces of its own vocabulary, and every \
             vocabulary disagrees."
        );
        println!(
            "  The number that matters is the one of the tokenizer your model \
             actually uses."
        );
    }
}
