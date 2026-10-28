# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- The dashboard shows the hand drawn logo of the project. The artwork is stored
  the way it was written down, in `tokenmaxxing-cli/assets/tokenmaxxing_logo.txt`,
  and is parsed, scaled down and turned into ratatui spans at startup.
- The logo comes in three sizes (111, 74 and 56 columns), the sharpest one
  that fits into the terminal is used, and a terminal that is too narrow for
  the smallest one gets the text `TokenMaxxing` instead.
- Interactive terminal dashboard (`tokenmaxxing ui`) built with `ratatui` and `crossterm`: an ASCII banner, repository analytics, cost estimates, a model comparison, optimization suggestions and provider usage.
- Six dashboard panels with a keyboard interface, a search field, sortable file lists, a help screen and a diff preview for proposed changes.
- `analysis` module in the core crate that counts the files of a tree individually, so the dashboard does not have to implement its own scanning.
- `usage` module in the core crate with a provider neutral usage record, a file store for the history and an empty store that reports "no usage" instead of inventing numbers.
- `CountKind::short_label` for table columns.
- The `tui` feature of the CLI crate. A build with `--no-default-features` keeps the terminal dependencies out and prints a short message instead of starting the dashboard.
- `CountKind` provenance on every token count (`Estimated`, `Approximate`, `ExactLocal`, `ProviderApi`), so an estimate can never be presented as an exact number.
- `LocalTokenizer` trait for tokenizers with a real vocabulary, and `TokenEngine::with_tokenizer` for exact local counts.
- `TokenCountProvider` trait plus `ProviderError` as a provider-neutral interface for future Anthropic and OpenAI adapters.
- OpenAI tokenizers based on the published byte-pair-encoding vocabularies (`cl100k_base`, `o200k_base`, `r50k_base`, `p50k_base`), bundled with the crate so counting works offline.
- Anthropic token counting through the provider's own `messages/count_tokens` endpoint, behind an `HttpTransport` trait so it can be tested without a network.
- `TokenizerInfo` and `ProviderInfo` record the tokenizer identity, version, encoding, supported models, vocabulary source and the input types that are not supported.
- CLI: `count --mode estimate|local|api|compare`, `count --provider`, `count --model`, the `cost` command and the `tokenizers` command.
- `pricing` module with dated, externally configured `PricingProfile` files, separate rates for input, output, cached input and cache write tokens, and the `cost` module with the optimization report.
- Streaming, tokenizer and provider regression tests, including chunk-size invariance, UTF-8 boundary coverage and the reference token ids of the published vocabularies.
- An ignored benchmark that compares the real tokenizer with the estimators.

### Changed
- Streaming tokenization decodes UTF-8 incrementally and carries incomplete multi-byte sequences across read boundaries instead of inserting replacement characters.
- Streaming tokenization keeps pieces aligned across chunk boundaries, so streaming a text returns the same count as counting the text at once.
- `TokenCount.chars` now counts Unicode characters instead of UTF-8 bytes; `TokenCount.bytes` was added for the byte length.
- The structural estimator attaches whitespace to the word that follows it, which brings it closer to real byte-pair-encoding tokenizers.
- `Accurate` mode is documented and labelled as an approximation rather than an exact count.
- CLI and dashboard output state the count method next to every token number.
- The optimizer reports what it removed, step by step.

### Fixed
- An exact count request is never silently downgraded to an estimate: asking for a local Claude count or an OpenAI API count fails with an explanation of what is available instead.
- The repository analysis no longer panics on a file whose head is cut in the middle of a multi byte character. The head is taken by characters now.
- The dashboard resolves the paths of a proposed change against the scanned root, so a change no longer lands in the working directory of the process.

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
