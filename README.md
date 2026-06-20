# TokenMaxxing

[![Rust](https://img.shields.io/badge/rust-1.96%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Release](https://img.shields.io/badge/release-v0.5.0--beta-orange.svg)](CHANGELOG.md)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](STABILIZATION.md)

TokenMaxxing is a lightweight repository intelligence and context optimization toolkit built in Rust. It is designed to reduce unnecessary token consumption and optimize prompt structures before injection into LLMs (such as Claude Code) through codebase analysis, duplicate detection, and semantic context compression.

> [!NOTE]
> This project is currently in **Public Beta**. While fully functional and verified, APIs and options may evolve based on real-world usage.

---

## Features

- **Token Counting Engine**: Fast, Accurate, and Hybrid subword token estimation using parallel directory walkers.
- **Smart Repository Heuristics**: Detects build artifacts (e.g., target, build, dist), package locks, duplicate files, cache temp folders, and oversized structures.
- **Context Quality Ranker**: Separates file contexts into logical blocks and ranks them by semantic value.
- **Context Simulator**: Models repository size ratio against typical LLM context bounds (200k tokens) to alert users of context overflow risks.
- **Prompt Rewrite Engine**: Compresses conversational noise using mode-specific telegraphic filters.
- **Memory Generator**: Caches dependency schemas and codebase architecture summaries to avoid repeated file walks.
- **Visual Dashboards**: Renders output metrics cleanly into Markdown, JSON, or static HTML layouts.

---

## Installation

TokenMaxxing CLI can be built and installed locally via Cargo:

```bash
cargo install --path tokenmaxxing-cli
```

---

## Quick Start

Initialize a quick scan of your current directory:

```bash
# Run the end-to-end auto pipeline
tokenmaxxing auto .

# Analyze repository waste
tokenmaxxing waste .

# Evaluate context window space utilization
tokenmaxxing simulate .
```

---

## Command Reference

| Command | Action |
| :--- | :--- |
| `count <PATH>` | Counts tokens in a file or directory using fast or accurate modes. |
| `optimize <PATH>` | Minifies Markdown, JSON, or XML formatting. |
| `compress <PATH>` | Trims redundant whitespace and boilerplate segments. |
| `analyze <PATH>` | Logs oversized structures and suspected duplicated files. |
| `stats <PATH>` | Outputs total estimated tokens using the hybrid algorithm. |
| `report <PATH>` | Compiles an optimization report as Markdown dashboard. |
| `benchmark <PATH>` | Runs local execution speed benchmarks and saves metrics. |
| `context <PATH>` | Renders quality metrics, noise ratio, and optimization opportunities. |
| `rank <PATH>` | Lists chunk prioritizations sorted by semantic logic. |
| `waste <PATH>` | Tallies cache files, target artifacts, and duplicate content bytes. |
| `simulate <PATH>` | Models footprint against LLM limits and alerts overflow risk. |
| `auto <PATH>` | Executes a sequential pipeline scan, waste test, and scoring pass. |
| `rewrite <PATH>` | Condenses prompt politeness blocks (Balanced, Aggressive flags). |
| `summarize <PATH>` | Evaluates active module types and component footprints. |
| `memory <PATH>` | Tracks parsed cargo dependency vectors and workspace layouts. |
| `dashboard <PATH>` | Outputs full metrics to terminal, `--html` file, or `--json` parser. |

---

## Architecture Overview

TokenMaxxing segregates logic across:
- `tokenmaxxing-core`: The main processing and estimation engine.
- `tokenmaxxing-cli`: Interactive CLI.
- Multi-language bindings (`napi-rs` for JS/TS, `pyo3` for Python).

---

## Benchmark Highlights
Evaluated on the local repository structure:

| Metric | Performance |
| :--- | :--- |
| **Counting Speed** | 43 ms |
| **Compression Ratio** | 67.74% (lower is better) |
| **Optimization Ratio** | 68.15% (lower is better) |
| **Repository Analysis Speed** | 4 ms |

---

## Validation Summary
Verified against standard tests:
- **Unit Tests**: 15 tests (all passing).
- **Quality Checks**: `cargo clippy` and `cargo fmt` verified (0 warnings, 0 errors).
- **Release Compilation**: Successful.

---

## Roadmap

### Current Version (v0.5.0-beta)
- [x] Context Quality scoring and chunk ranking algorithms.
- [x] Smart repository analysis (Target builds, lockfiles, cache paths).
- [x] Interactive Prompt Rewriter modes.
- [x] Dashboard generation (JSON/HTML/Markdown).

### Future Milestones
- [ ] Tree-sitter advanced AST compression support.
- [ ] Pre-commit git hooks integration.

---

## Contributing
Please refer to [CONTRIBUTING.md](CONTRIBUTING.md) for local setup and testing standards.
