//! Benchmark of the real tokenizer against the estimators.
//!
//! The benchmark is ignored by default because it measures the machine it
//! runs on. Run it with:
//!
//! ```text
//! cargo test --release --test tokenizer_benchmark -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;
use tokenmaxxing_core::engine::{EstimationMode, TokenEngine};
use tokenmaxxing_core::tokenizer::openai::OpenAiTokenizer;
use tokenmaxxing_core::tokenizer::LocalTokenizer;

/// How often every measurement is repeated.
const RUNS: usize = 5;

/// The files that represent a typical project.
fn representative_files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![
        root.join("../README.md"),
        root.join("../Cargo.toml"),
        root.join("src/engine.rs"),
        root.join("src/tokenizer/openai.rs"),
        root.join("src/provider.rs"),
    ];

    files.retain(|file| file.exists());
    files
}

/// Reads the files and returns their content and the total size.
fn read_files(files: &[PathBuf]) -> (Vec<String>, usize) {
    let mut contents = Vec::new();
    let mut bytes = 0;

    for file in files {
        let content = std::fs::read_to_string(file).expect("the file has to be readable");
        bytes += content.len();
        contents.push(content);
    }

    (contents, bytes)
}

/// Peak resident set size in kilobytes, when the platform reports it.
fn peak_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("VmHWM:")?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// Runs a measurement and prints it.
fn measure(name: &str, megabytes: f64, count: &mut dyn FnMut() -> usize) -> usize {
    let start = Instant::now();
    let mut tokens = 0usize;
    for _ in 0..RUNS {
        tokens = count();
    }
    let elapsed = start.elapsed().as_secs_f64() / RUNS as f64;
    let throughput = if elapsed > 0.0 {
        megabytes / elapsed
    } else {
        f64::INFINITY
    };

    println!(
        "{name:<28} {tokens:>8} tokens  {:>8.3} ms  {throughput:>9.1} MB/s",
        elapsed * 1000.0
    );

    tokens
}

#[test]
#[ignore = "measures the machine it runs on"]
fn test_benchmark_tokenizer_against_the_estimators() {
    let files = representative_files();
    let (contents, bytes) = read_files(&files);

    assert!(!contents.is_empty(), "no files to benchmark");
    let megabytes = bytes as f64 / 1_000_000.0;

    println!();
    println!("Benchmark of {} files, {} bytes", files.len(), bytes);
    println!("{:-<78}", "-");

    let fast = TokenEngine::new(EstimationMode::Fast);
    let structural = TokenEngine::new(EstimationMode::Accurate);
    let tokenizer = OpenAiTokenizer::for_model("gpt-4o").expect("gpt-4o has a tokenizer");

    let fast_tokens = measure("ratio estimate", megabytes, &mut || {
        contents
            .iter()
            .map(|text| fast.count_text(text).tokens)
            .sum()
    });
    let structural_tokens = measure("structural approximation", megabytes, &mut || {
        contents
            .iter()
            .map(|text| structural.count_text(text).tokens)
            .sum()
    });
    let exact_tokens = measure("tiktoken o200k_base", megabytes, &mut || {
        contents
            .iter()
            .map(|text| tokenizer.count_tokens(text))
            .sum()
    });

    println!("{:-<78}", "-");
    println!(
        "ratio estimate      {:>7.2}% against the tokenizer",
        (fast_tokens as f64 - exact_tokens as f64) / exact_tokens as f64 * 100.0
    );
    println!(
        "structural approx.  {:>7.2}% against the tokenizer",
        (structural_tokens as f64 - exact_tokens as f64) / exact_tokens as f64 * 100.0
    );
    if let Some(rss) = peak_rss_kb() {
        println!("peak resident set:  {rss} kB");
    }
    println!();
}
