# TokenMaxxing V2 Architecture

TokenMaxxing V2 extends the core BPE-based token count structures with context quality metrics, repository waste analysis, prompt rewrites, and pipeline orchestration.

```
                    +------------------------------------+
                    |        TokenMaxxing CLI            |
                    +------------------------------------+
                                       |
                                       v
                    +------------------------------------+
                    |       tokenmaxxing-core            |
                    +------------------------------------+
                     /        |          |          \    \
                    /         |          |           \    \
                   v          v          v            v    v
             +---------+ +---------+ +---------+ +-------+ +---------+
             | Engine  | | Context | | Repo    | | Sim   | | Rewrite |
             +---------+ +---------+ +---------+ +-------+ +---------+
                  |           |           |          |          |
                  +-----------+-----------+----------+----------+
                                       |
                                       v
                              +-----------------+
                              |  Auto Pipeline  |
                              +-----------------+
                                       |
                                       v
                              +-----------------+
                              |    Dashboard    |
                              +-----------------+
```

## System Layout

1. **Core Processing Engine**:
   - `engine.rs`: Token calculations and repository-wide counts.
   - `context.rs`: Context ranking and quality evaluations.
   - `repository.rs`: Smart waste analysis and repository memory mapping.

2. **Orchestration**:
   - `simulator.rs`: Token space analysis.
   - `pipeline.rs`: Runs structural scanning.
   - `rewrite.rs`: Applies prompt compressions.
   - `dashboard.rs`: Renders results.
