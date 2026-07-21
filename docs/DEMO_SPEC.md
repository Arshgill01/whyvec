# Demonstration specification

## Narrative

The demonstration shows a compiler message that describes a symptom, a counterfactual experiment that reveals a tested sufficient condition, and a repository-aware agent that refuses the unsafe shortcut before implementing and validating a defensible repair.

## Fixture

Use a performance-relevant C/FFI-style kernel with multiple arrays and a count passed by pointer:

```c
void add_vectors_(int *output, const int *input, const int *count) {
    for (int i = 0; i < *count; ++i)
        output[i] += input[i];
}
```

The fixture includes callers and tests where `count` may overlap `output`, making unconditional bound caching or an unjustified `restrict` annotation observably incorrect.

## Required beats

### Baseline

- Show the real Clang invocation and version.
- Show the selected source loop.
- Show the serialized optimization result and concise missed-vectorization remark.
- Avoid claiming the compiler identified the actual source contract.

### Counterfactual diagnosis

- Run `whyvec explain-opt` with the explicit source/function/parameter mapping
  and the fingerprinted LLVM transformer and identity helper, then run
  `whyvec derive-obligation` on the retained positive report.
- Show baseline and singleton variants.
- Highlight that `count modeled noalias` changes the matched loop to vectorized.
- State that this is a tested sufficient condition under the recorded pipeline.
- Show the candidate obligation, supporting access evidence, preconditions, and
  residual unsupported semantics.

### Agent decision

- Show Codex inspecting declarations, callers, and tests.
- Show it considering and rejecting global `restrict` because caller evidence is incomplete.
- Show it rejecting unconditional bound caching because an overlap case changes behavior.
- Show it selecting guarded versioning or refusing if the guard cannot be represented safely.

### Adversarial behavior

- Run a non-overlap case through the optimized branch.
- Run a case where `count` aliases writable output through the unchanged fallback.
- Show both results match the original behavior.

### Compiler and performance evidence

- Show the repaired fast path's optimization record.
- Show raw benchmark summary generated from retained samples.
- State the measured environment and avoid universal speed claims.

### Close

Use the product statement:

> Compiler remarks tell an agent what happened. WhyVec runs controlled experiments to discover what tested condition changes the outcome, then gives Codex the obligation it must enforce or refuse.

## Demonstration integrity

- Record against the same pinned environment judges can run.
- Do not splice outputs from different compiler versions or targets.
- Keep commands readable and results reproducible.
- Include a refusal fixture in the repository even if the primary narrative shows a successful repair.
- Use only benchmark numbers produced by the committed harness and retained raw output.
