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

- **Token Counting Engine**: Fast, Accurate, and Hybrid subword token estimation using parallel directory walkers. Every count reports how it was produced, so an estimate is never presented as an exact number.
- **Real Tokenizers**: Exact counts with the published OpenAI byte-pair-encoding vocabularies (`cl100k_base`, `o200k_base`, `r50k_base`, `p50k_base`). The vocabularies are bundled, so counting works offline.
- **Provider Counting**: Exact counts for Claude models through Anthropic's own `messages/count_tokens` endpoint. Anthropic does not publish Claude's tokenizer, so there is no local tokenizer for it.
- **Count Provenance**: Counts are labelled as `estimated`, `approximate`, `exact (local tokenizer)` or `exact (provider API)`. An exact count is only produced when a real tokenizer vocabulary or a provider API backs it.
- **Cost Estimation**: Dated, externally configured model pricing profiles with separate rates for input, output, cached input and cache write tokens.
- **Interactive Dashboard**: A terminal dashboard with an ASCII banner, repository analytics, cost estimates, a model comparison and safe optimization workflows. Every change needs an explicit confirmation.
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

# Open the interactive dashboard
tokenmaxxing ui .
```

---

## Command Reference

| Command | Action |
| :--- | :--- |
| `count <PATH>` | Counts tokens in a file or directory using fast or accurate modes and reports the count method. |
| `count <PATH> --mode local --provider openai` | Counts exactly with the published OpenAI vocabulary. |
| `count <PATH> --mode api --provider anthropic` | Counts exactly through Anthropic's counting endpoint. |
| `count <PATH> --mode compare` | Runs every available method and compares the results. |
| `tokenizers` | Lists the tokenizers and models that can be counted exactly. |
| `cost <PATH> --pricing <FILE>` | Reports the token and cost effect of optimizing a prompt. |
| `ui [PATH]` | Opens the interactive dashboard. |
| `optimize <PATH>` | Minifies Markdown, JSON, or XML formatting. |
| `compress <PATH>` | Trims redundant whitespace and boilerplate segments. |
| `analyze <PATH>` | Logs oversized structures and suspected duplicated files. |
| `stats <PATH>` | Outputs total estimated tokens using the hybrid algorithm and the count method. |
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

## Token Counting

```bash
# Heuristic estimate, no tokenizer is involved
tokenmaxxing count README.md

# Exact count with the published OpenAI vocabulary (works offline)
tokenmaxxing count README.md --mode local --provider openai --model gpt-4o

# Exact count for a Claude model through the Anthropic API
export ANTHROPIC_API_KEY=...
tokenmaxxing count README.md --mode api --provider anthropic --model claude-opus-5-5

# Compare every method that is available
tokenmaxxing count README.md --mode compare
```

### Model coverage

| Provider | Counting | Models |
| :--- | :--- | :--- |
| OpenAI | local tokenizer, exact | `cl100k_base`: gpt-4, gpt-4-32k, gpt-4-turbo, gpt-3.5-turbo, gpt-35-turbo. `o200k_base`: gpt-4o, gpt-4.1, gpt-4.5, o1, o3, o4-mini, gpt-5 and the Codex variants. `r50k_base`: gpt2. `p50k_base`: text-davinci-001. |
| Anthropic | provider API, exact for the model | every model the Anthropic API accepts, for example `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`. |

Dated model names such as `gpt-4o-2024-08-06` resolve to the same encoding as their family.

There is **no local tokenizer for Claude**. Anthropic does not publish its tokenizer, and a generic byte-pair-encoding tokenizer is not a substitute: `tokenmaxxing count --mode local --provider anthropic` fails with an explanation instead of returning a number that looks exact.

---

## Interactive Dashboard

```bash
tokenmaxxing ui                       # the current directory
tokenmaxxing ui /path/to/repository   # another directory
tokenmaxxing ui . --pricing pricing.json --model gpt-4o
tokenmaxxing ui . --usage ~/.local/share/tokenmaxxing/usage.jsonl
```

The dashboard is a terminal application. It draws six panels and never writes
to a file without an explicit confirmation.

```
                TokenMaxxing      <- the extrusion, one column to the right
               TokenMaxxing       <- the front face, bright white
     TOKEN ANALYSIS · COST OPTIMIZATION · BETTER CONTEXT
```

The banner is a minimal wordmark: the name of the project in a crisp front
face with a restrained red extrusion behind it. It takes two rows, the
subtitle below it makes three, so the panels keep their room. A terminal
that is too narrow for the wordmark gets the plain text `TokenMaxxing`.

### Keyboard shortcuts

| Key | Action |
| :--- | :--- |
| `j` / `k`, `down` / `up` | Move inside the focused panel |
| `tab`, `l` / `h` | Switch the focused panel |
| `enter` | Open the selection (file details, or the preview of a change) |
| `/` | Search the file list |
| `s` | Change the sort order of the file list |
| `r` | Scan again |
| `?` | Open and close the help |
| `q` / `esc` | Go back, leave the dashboard |

### What the panels mean

| Panel | Content |
| :--- | :--- |
| **Overview** | Files scanned, bytes, characters, tokens, the method that produced the token count and the estimated cost when a pricing profile is loaded. |
| **Token usage** | The input and output tokens of the recorded usage and a chart of the history. Empty when there is no history. |
| **Largest files** | The files with the most tokens, their share of the total and their estimated cost. Sortable by tokens, size or path. |
| **Model comparison** | The same text counted with every supported tokenizer and with the estimators. Every row states the model, the tokenizer and the method. |
| **Optimization suggestions** | Large files, duplicated files and repeated instruction lines, each with the reason it was suggested and the expected saving. |
| **Provider usage** | The usage records a provider reported. The token counts of the repository are **not** provider usage and the panel says so. |

### Provenance of the numbers

- Token counts of the repository come from the token engine, and every panel
  states the method: `exact` (a real tokenizer), `estimate` or `approx`.
- The costs are estimates. They need a dated pricing profile and a model that
  the profile knows; without them the panel says that no price is available.
- The expected saving of a suggestion is an estimate until the change is
  applied and measured again.
- The usage history holds only counts, models and timestamps. No prompt, no
  file content and no credential is written to it.

### Applying a change

A suggestion never changes a file on its own. `enter` opens a preview with the
diff of the change, `enter` again applies it and `esc` cancels it. Removing a
duplicate only happens when the file that stays is still identical.

### Without the dashboard

Every command of the CLI works without the terminal interface, so scripts and
CI are not affected:

```bash
tokenmaxxing count . --mode local --provider openai
tokenmaxxing stats .
tokenmaxxing dashboard . --json
```

A build without the terminal dependencies is possible as well:

```bash
cargo build -p tokenmaxxing-cli --no-default-features
```

Such a build prints a short message instead of starting the dashboard.

### Known limitations

- The dashboard needs a real terminal. Without one it refuses to start and
  points at the commands that work in a script.
- A terminal below 64 columns, or one that is too short for the banner and
  the panels, shows the wordmark and a message instead of the panels.
- The wordmark needs 14 columns. Below 24 columns the banner falls back to
  the plain text `TokenMaxxing`.
- The scan reads the whole directory. On a very large repository the first
  scan takes a moment; the display stays responsive while it runs.
- The usage history is empty unless a provider integration writes to it. The
  tool does not know the usage of Claude Code or Codex sessions on its own.


---

## Cost Estimation

Cost estimation needs a pricing profile, because model prices change and differ per provider and per plan. The crate ships no prices at all.

```json
{
  "currency": "USD",
  "effective_date": "2026-10-01",
  "source": "https://your-provider.example/pricing",
  "models": {
    "gpt-4o": {
      "input_per_million_tokens": 2.5,
      "output_per_million_tokens": 10.0,
      "cached_input_per_million_tokens": 1.25
    }
  }
}
```

```bash
tokenmaxxing cost PROMPT.md --pricing pricing.json --model gpt-4o
```

The report states the token counts before and after the optimization, what was removed, the cost difference, and where every number comes from. It does not claim that a shorter prompt keeps the task quality, because nothing in the tool measures that.

---

## Token Count Accuracy

TokenMaxxing separates three kinds of token counts, and every result carries the
method that produced it:

| Method | Produced by | Trust |
| :--- | :--- | :--- |
| `estimated` | Character ratio heuristic (`Fast` and large `Hybrid` inputs) | Order of magnitude only. |
| `approximate` | Structural byte-pair-encoding emulation (`Accurate` and small `Hybrid` inputs) | Close to a real tokenizer, but no vocabulary is loaded. |
| `exact (local tokenizer)` | A tokenizer that ships a real vocabulary | Exact for that tokenizer. |
| `exact (provider API)` | A provider API that owns the authoritative tokenizer | Exact for that model. |

Rules the engine follows:

- A count is never labelled exact unless a real tokenizer or a provider API produced it.
- An exact request is never silently downgraded to an estimate. Asking for a local Claude count fails with an explanation.
- No model prices are hardcoded; cost estimation takes the rates from the caller.
- Streaming a text yields the same count as counting the whole text, independent of how the reader splits the bytes. UTF-8 sequences that straddle a read boundary are carried over instead of being corrupted.

---

## Architecture

TokenMaxxing segregates logic across:
- `tokenmaxxing-core`: The main processing and estimation engine.
- `tokenmaxxing-cli`: Interactive CLI.
- Multi-language bindings (`napi-rs` for JS/TS, `pyo3` for Python).

---

## Benchmarks

Counting the five representative project files of this repository (54 662 bytes), release build:

| Method | Tokens | Time | Throughput | Against the tokenizer |
| :--- | ---: | ---: | ---: | ---: |
| **Ratio estimate** | 15 608 | 0.003 ms | 15 812 MB/s | +21.24 % |
| **Structural approximation** | 17 086 | 0.282 ms | 193.9 MB/s | +32.72 % |
| **tiktoken `o200k_base`** | 12 874 | 3.582 ms | 15.3 MB/s | exact |

Peak resident set of the benchmark process: 53 876 kB.

The exact tokenizer is roughly 1 200 times slower than the ratio estimate and about 13 times slower than the structural approximation. That is the price of an exact number.

Reproduce it with:

```bash
cargo test --release --test tokenizer_benchmark -- --ignored --nocapture
```

---

## Validation Results
Verified against standard tests:
- **Unit Tests**: 144 tests (all passing), 2 live provider tests ignored by default.
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
- [ ] A provider integration that writes real usage records for the dashboard.

---

## Contributing
Please refer to [CONTRIBUTING.md](CONTRIBUTING.md) for local setup and testing standards.

---

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
