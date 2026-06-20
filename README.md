# TokenMaxxing

![Rust](https://img.shields.io/badge/Rust-Stable-orange?logo=rust)

![Status](https://img.shields.io/badge/Status-Beta-yellow)

![License](https://img.shields.io/badge/License-MIT-green)

![Version](https://img.shields.io/badge/Version-v0.5.0--beta-blue)

![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)

![GitHub Stars](https://img.shields.io/github/stars/YigitEfe-DEV/TokenMaxxing?style=social)

![GitHub Issues](https://img.shields.io/github/issues/YigitEfe-DEV/TokenMaxxing)

![GitHub Last Commit](https://img.shields.io/github/last-commit/YigitEfe-DEV/TokenMaxxing)

![GitHub Release](https://img.shields.io/github/v/release/YigitEfe-DEV/TokenMaxxing)

---

Repository intelligence and context optimization toolkit for Claude Code workflows.

TokenMaxxing helps developers reduce unnecessary token consumption through repository analysis, duplicate detection, context compression, prompt optimization, and context prioritization.

---

## Overview

**TokenMaxxing** is a high-performance **Rust CLI** and **developer tools** suite designed for codebase profiling, serving as a smart **Claude tokenizer** assistant and complete **repository intelligence** utility. Specially engineered for **Claude Code** workflows, it guarantees maximum **token efficiency**, precise **token optimization**, and rigorous **context optimization** by delivering automated **repository analysis**, telegraphic **prompt optimization**, and advanced **context compression** (such as whitespace minification and semantic deduplication).

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

## Architecture

TokenMaxxing segregates logic across:
- `tokenmaxxing-core`: The main processing and estimation engine.
- `tokenmaxxing-cli`: Interactive CLI.
- Multi-language bindings (`napi-rs` for JS/TS, `pyo3` for Python).

---

## Benchmarks
Evaluated on the local repository structure:

| Metric | Performance |
| :--- | :--- |
| **Counting Speed** | 43 ms |
| **Compression Ratio** | 67.74% (lower is better) |
| **Optimization Ratio** | 68.15% (lower is better) |
| **Repository Analysis Speed** | 4 ms |

---

## Validation Results
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

---

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
