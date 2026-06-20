# TokenMaxxing v0.5.0-beta Release Notes

Welcome to the **Public Beta** of TokenMaxxing! This release transitions TokenMaxxing from a simple tokenizer tool into a lightweight context optimization and repository performance platform.

---

## What's New

### 1. Context Maxxing Engine
Allows developers to measure how useful their context files are before injecting them into prompt windows. It features:
* **Quality & Density Scores**: Estimates word/token densities and reports repetitive sentence noise ratios.
* **Semantic Prioritization**: Splits files into chunks and prioritizes logic blocks (like classes and functions) over licence templates and generic filler.

### 2. Smart Repository Analyzer
Scans directory contents to find optimization targets:
* Filters lockfiles, temporary buffers, caches, and build targets (`/target`, `/dist`, `/build`).
* Finds identical file duplicates and tracks overall codebase efficiency.

### 3. Context Simulator
Predicts how much of an LLM's context window (e.g. 200k tokens for Claude) your repository consumes, warning against potential overflow risks.

### 4. Prompt Rewrite Engine
Features `Conservative`, `Balanced`, and `Aggressive` modes to strip politeness filters and conversational text from prompts.

### 5. Automated Pipeline
Orchestrates scanning, analysis, rankings, and simulations in a single unified command: `tokenmaxxing auto <PATH>`.

---

## Validation & Benchmarks

All V2 modules have been fully verified inside the codebase:
- **Unit Tests**: 15 tests verified passing (`cargo test --all`).
- **Formatting & Quality**: Passes `cargo clippy --all-targets --all-features` and `cargo fmt --check` with 0 warnings.
- **Speed Benchmarks**:
  - Counting Speed: **43 ms**
  - Repository Analysis Speed: **4 ms**
  - Compression Ratio: **67.74%**
  - Optimization Ratio: **68.15%**

---

## Known Limitations

- **Heuristic Hash Scans**: Duplicate detection uses length + 100 character header matching. Though highly performant, it is not a cryptographically strong hash.
- **Rules-Based Classifier**: Semantic prioritize scores use static keyword indicators (like checking for functions or classes) and lack NLP embedding verification.
- **Binding Limits**: JavaScript and Python bindings cover core counting and compression operations, but do not yet expose full repository analytics pipelines.

---

## Next Milestones

- **Tree-sitter Compression**: Introduce syntax tree parsing for AST-aware line stripping.
- **Git Hook Integration**: Expose pre-flight commit integrations to auto-warn developers if lockfiles are staged.
