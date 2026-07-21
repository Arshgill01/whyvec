# Demonstration specification

## Executable entrypoint

`./scripts/demo` runs this specification from a disposable repository using
live compiler output and a fresh installed-plugin Codex session. It builds the
real CMake/Ninja project, emits the agent packet, streams observable Codex
commands and patch events, validates the exact generated source, and finishes
with a fast refusal. Nothing in the terminal result is prerecorded.

`./scripts/demo --ci` is the credential-free CI/judge verification path. It
uses the byte-identical candidate retained from the actual session and reruns
all deterministic compiler, behavior, sanitizer, benchmark, and mutation
checks. The distinction is printed explicitly.

The environment can be reproduced with:

```console
containers/judge/build.sh
```

The image pins Ubuntu 24.04 by digest, Rust 1.96.1, Clang/LLVM 21, Codex CLI
0.144.3, Python dependencies, the built helpers, and the installed WhyVec
plugin.

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

- Run `whyvec analyze demo/src/kernel.c:4` through the real CMake/Ninja
  compilation database with automatic function, parameter, helper, and
  obligation resolution.
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
- Generate 3,271 defined-behavior executions with 1,123 fast and 2,148 fallback
  witnesses, including every in-range bound position for counts 2–65.
- Reject all eleven retained unsafe mutations before accepting the candidate.

### Compiler and performance evidence

- Show the repaired fast path's optimization record.
- Show raw benchmark summary generated from retained samples.
- State the measured environment and avoid universal speed claims.
- Treat a noisy distribution as a refusal, never as a weak success.

### Close

Use the product statement:

> Compiler remarks tell an agent what happened. WhyVec runs controlled experiments to discover what tested condition changes the outcome, then gives Codex the obligation it must enforce or refuse.

## Demonstration integrity

- Record against the same pinned environment judges can run.
- Do not splice outputs from different compiler versions or targets.
- Keep commands readable and results reproducible.
- Include a refusal fixture in the repository even if the primary narrative shows a successful repair.
- Use only benchmark numbers produced by the committed harness and retained raw output.
- Keep the actual-model record free of hidden reasoning, token telemetry,
  secrets, and machine-private paths.
