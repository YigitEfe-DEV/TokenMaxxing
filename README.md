# TokenMaxxing

<div align="center">
  <h3>A production-grade, open-source platform maximizing Claude Code efficiency.</h3>
</div>

## Vision
TokenMaxxing reduces unnecessary token consumption while preserving output quality. It moves past naive token counting and introduces context compression, instruction deduplication, AST-aware repository scanning, and semantic pruning to maximize useful context per token.

## Features
- **Token Engine**: Fast, Accurate, and Hybrid modes. Batch processing and streaming tokenizer.
- **Prompt Optimizer**: Minifies Markdown, JSON, XML, YAML. Deduplicates instructions.
- **Context Compressor**: Semantic chunking and recursive summarization for massive contexts (40-70% token reduction).
- **Repository Intelligence**: Analyzes codebases to flag repetitive templates, bloated docs, and duplicated code.
- **Claude Code Accelerator**: First-class CLI (`tokenmaxxing`) to execute optimizations instantly.
- **Agent Optimization Layer**: Analyzes and streamlines AI agent workflows.

## Benchmarks
*Currently in development. Target:*
* 20-60% token reduction on average repositories.
* 2x faster analysis than naive implementations.
* Native support for 100k+ files.

## Installation

### CLI
```bash
cargo install tokenmaxxing-cli
```

### Python Bindings
```bash
pip install tokenmaxxing
```

### TypeScript Bindings
```bash
npm install tokenmaxxing
```

## Usage Examples

```bash
# Get stats for current repository
tokenmaxxing stats .

# Optimize a specific prompt
tokenmaxxing optimize prompt.md

# Compress a repository context
tokenmaxxing compress .
```

## Architecture

TokenMaxxing is built on a modular architecture powered by a Rust core.

- `tokenmaxxing-core`: The high-performance engine for parsing and processing.
- `tokenmaxxing-cli`: Command-line interface.
- Bindings expose the engine to Python/TypeScript ecosystems.

*(See [ARCHITECTURE.md](ARCHITECTURE.md) for full details)*

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct, and the process for submitting pull requests to us.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
