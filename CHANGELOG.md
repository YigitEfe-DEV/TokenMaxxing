# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `CountKind` provenance on every token count (`Estimated`, `Approximate`, `ExactLocal`, `ProviderApi`), so an estimate can never be presented as an exact number.
- `LocalTokenizer` trait for tokenizers with a real vocabulary, and `TokenEngine::with_tokenizer` for exact local counts.
- `TokenCountProvider` trait plus `ProviderError` as a provider-neutral interface for future Anthropic and OpenAI adapters. No live API calls are implemented in this phase.
- `pricing` module with `ModelPricing`, `PricingProvider` and `estimate_cost`. No model prices are hardcoded; rates are supplied by the caller.
- Streaming and tokenizer regression tests, including chunk-size invariance and UTF-8 boundary coverage.

### Changed
- Streaming tokenization decodes UTF-8 incrementally and carries incomplete multi-byte sequences across read boundaries instead of inserting replacement characters.
- Streaming tokenization keeps pieces aligned across chunk boundaries, so streaming a text returns the same count as counting the text at once.
- `TokenCount.chars` now counts Unicode characters instead of UTF-8 bytes; `TokenCount.bytes` was added for the byte length.
- The structural estimator attaches whitespace to the word that follows it, which brings it closer to real byte-pair-encoding tokenizers.
- `Accurate` mode is documented and labelled as an approximation rather than an exact count.
- CLI and dashboard output state the count method next to every token number.

## [0.5.0-beta] - 2026-06-20

### Added
- Context Maxxing Engine (quality scoring, chunk relevance ranking, prioritizer).
- Smart Repository scanning heuristics (generated lockfiles, target/build directories, cache files, oversized structures).
- Context Simulator modeling token window limits and overflow risk.
- Auto pipeline automation flow.
- Prompt Rewrite modes (Conservative, Balanced, Aggressive prompt compression).
- Repository memory layout mapping dependency arrays.
- Dashboard report system outputting JSON, HTML, and Markdown performance charts.
- Benchmark validation framework tracking timing and compression scores.
- Documented complete command suites and localized Cargo structures.
- Added GitHub badges and improved repository presentation.

## [0.1.0] - 2026-06-20

### Added
- Scaffolded BPE token estimation logic.
- Base CLI implementation.
