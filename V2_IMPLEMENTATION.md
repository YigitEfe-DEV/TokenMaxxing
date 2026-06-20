# TokenMaxxing V2 Implementation

This document covers details of the **TokenMaxxing V2** features implemented to transition the project from a simple tokenizer utility into a context optimization platform.

## Features Implemented

### 1. Context Maxxing Engine (`context.rs`)
* **Relevance & Quality Scores**: Measures noise patterns, repeating word counts, and outputs a quality metrics score.
* **Semantic Ranking**: Separates the file contents into logical units and ranks them by structural markers (definitions, TODO tasks, and boilerplate).
* **Opportunity Spotting**: Identifies dead context blocks and duplication.

### 2. Smart Repository mode (`repository.rs`)
* **Waste Scan**: Automatically filters and classifies repository junk (build outputs like `/target` and `/dist`, package lockfiles, temp buffers, vendor code).
* **Efficiency Meter**: Scores overall repo efficiency based on waste volume vs actual code volume.

### 3. Context Simulator (`simulator.rs`)
* Models the repository footprint against Claude's 200k token limits.
* Computes overflow risks and prints recommended mitigation steps.

### 4. Auto Pipeline (`pipeline.rs`)
* Automates scanning, analysis, ranking, and optimization in a single workflow.

### 5. Rewrite Engine (`rewrite.rs`)
* Politeness filter removes conversational noise.
* Conservative, Balanced, and Aggressive compression settings simplify prompts.

### 6. Repository Memory System (`repository.rs`)
* Captures a structural architecture summary and dependency map to reduce repeated codebase reads.

### 7. Performance Dashboard (`dashboard.rs`)
* Renders metrics cleanly into Markdown, static HTML, or serializable JSON.
