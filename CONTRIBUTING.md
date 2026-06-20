# Contributing to TokenMaxxing

First off, thanks for taking the time to contribute! TokenMaxxing is built to maximize Claude's token efficiency. 

## Development Workflow

1. **Research First**: Before implementing a feature, ensure it is thoroughly researched.
2. **No Placeholder Implementations**: Every module pushed must be functional. 
3. **No TODOs**: If it's worth a TODO, it's worth implementing or filing an issue for.
4. **Testing Requirements**:
   - Write Unit tests for logic.
   - Integration tests for cross-module features.
   - Benchmark tests to prove token/latency optimizations.
   - Stress tests for massive repositories.

## Coding Standards

- The core is built in **Rust**. Follow `rustfmt` and `clippy` standards.
- Documentation must be provided for all public APIs.
- Python and TS bindings must maintain the same API behavior.

## Pull Request Process

1. Ensure your code passes all tests (`cargo test`).
2. Run benchmarks and include the output in your PR description.
3. Update the `CHANGELOG.md`.
