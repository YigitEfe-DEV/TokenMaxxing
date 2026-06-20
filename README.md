# TokenMaxxing

<div align="center">
  <h3>A production-grade, open-source context optimization platform maximizing Claude Code efficiency.</h3>
</div>

## Vision
TokenMaxxing reduces unnecessary token consumption while preserving output quality. It moves past naive token counting and introduces context quality scoring, smart repository waste detection, prompt rewriting, and automated performance pipelines to maximize useful context per token.

## V2 Platform Architecture
TokenMaxxing is built as a highly performant Rust engine:
- `tokenmaxxing-core`: The modular context ranking and optimization engine.
- `tokenmaxxing-cli`: Interactive CLI.
- Multi-language bindings for Python and TypeScript ecosystems.

## V2 Feature Suite
- **Token Engine**: Fast, Accurate, and Hybrid counting modes with parallel rayon traversal.
- **Context Maxxing**: Context ranking, noise ratio evaluations, quality scoring, and redundancy detection.
- **Smart Repository Mode**: Flags generated lockfiles, build target dirs, caches, and duplicate files.
- **Context Simulator**: Simulates workspace sizes against context windows (e.g. Claude 200k), evaluating overflow risks.
- **Rewrite Engine**: Condenses prompts using politeness filters and mode-specific settings (Conservative, Balanced, Aggressive).
- **Repository Memory**: Generates architecture and dependency summaries to avoid expensive rescans.
- **Performance Dashboard**: Renders insights into Markdown, static HTML, or JSON.

## Installation

### CLI
```bash
cargo install tokenmaxxing-cli
```

## CLI Usage

### Backward Compatible Core Commands:
```bash
# Count tokens in a file or directory
tokenmaxxing count .

# Minify prompt files (Markdown, JSON, XML)
tokenmaxxing optimize prompt.md

# Compress semantic chunks
tokenmaxxing compress .
```

### V2 Platform Commands:
```bash
# Smart repository waste analysis
tokenmaxxing waste .

# Evaluate context scores and noise ratios
tokenmaxxing context prompt.md

# Rank semantic chunks by priority
tokenmaxxing rank prompt.md

# Simulate context utilization and overflow risk
tokenmaxxing simulate .

# Rewrite prompts into telegraphic statements
tokenmaxxing rewrite prompt.md --aggressive

# Run the automated performance pipeline
tokenmaxxing auto .

# View structural memory
tokenmaxxing memory .

# Render Markdown, HTML, or JSON dashboards
tokenmaxxing dashboard . --html
```

## License
MIT License.
