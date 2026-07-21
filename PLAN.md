# WhyVec execution plan

This is the living implementation plan. Phase documents contain detailed acceptance gates; this file records ordering, dependencies, and current state.

## Status vocabulary

- `queued` — dependency gates are not yet satisfied.
- `ready` — dependency gates are satisfied and work may begin.
- `active` — implementation or validation is underway.
- `verifying` — implementation exists and exit gates are being evaluated.
- `complete` — every exit gate has evidence.
- `blocked` — a named external or semantic condition prevents progress.
- `superseded` — a later architectural decision replaces the phase contract.

## Current state

| Phase | Capability | Status | Evidence |
| --- | --- | --- | --- |
| 00 | Original vectorization foundation | superseded | [ADR 0005](docs/decisions/0005-causal-compiler-debugger.md) broadens the shared core |
| R1 | Generic compiler-question and experiment domain | active | Query, observation, intervention, three-valued oracle, and pipeline-fidelity types compile |
| R2 | Per-adapter toolchain and fixture system | active | Clang 21 and rustc 1.96.1/LLVM 22 fixtures execute; Rust split pipeline remains labeled surrogate |
| R3 | Immutable counterfactual experiment runtime | complete | Build and optimization adapters share deterministic three-valued search, bounded argv-only processes, create-new artifact retention, digest verification, read-only finalization, and semantic replay with tamper refusal |
| R4 | LLVM optimization-causality pack | complete | Public `explain-opt`/`replay-opt` provide verified typed deltas, equivalent-confirmed pipeline replay, stable loop identity, retained ambiguity and no-success declines, deterministic finite search, confirmation runs, immutable evidence, and semantic reproduction |
| R5 | Build-causality query | complete | Cargo/rustc stable diagnostic identity, parsed Rust syntax-item grouping with exact-hunk fallback, deterministic nested search, isolated Bubblewrap worktrees, cascade/removal witnesses, immutable reports, CLI, and semantic replay pass positive, interacting, ambiguity, tamper, and hostile-build fixtures |
| R6 | C++, Rust, TypeScript, and GCC adapter expansion | complete | C++ linkage/template positives and macro ambiguity pass through the LLVM pack; Cargo, Clang, GCC, and TypeScript build adapters pass public schema/replay checks; GCC native optimization records and integrity-checked LLVM comparison pass replay and tamper refusal |
| R7 | Source-action, validation, and measurement | complete | The derived C bound obligation drives a checked `uintptr_t` guard with cached-bound fast path and unchanged fallback; 9 differential executions, both branch witnesses, overflow refusals, ASan/UBSan, branch-specific optimization records, 31-sample distributions, environment capture, and noise-aware statistics are retained |
| R8 | GPT-5.6/Codex repository engineering loop | complete | The [R3–R8 completion audit](docs/R3_R8_COMPLETION_AUDIT.md) maps replayed evidence, exact candidate linkage, complete behavior/compiler/performance gates, unsafe-alternative and noise refusal, clean-checkout validation, and a green pinned-sandbox workflow |

## Workstreams

### Counterfactual experiment engine

- Resolve frontend/build adapters and normalize commands without shell interpretation.
- Represent build diagnostics, optimization decisions, and divergences as typed observations.
- Represent patch atoms and compiler assumptions as typed interventions.
- Use a three-valued oracle and retain unresolved variants without converting them to negative evidence.
- Fingerprint compiler inputs and retain raw optimization records.
- Establish stable observation identity across variants.
- Generate one-delta shadow variants.
- Search declared intervention sets deterministically.
- Detect confounded experiments and decline ambiguous comparisons.

### Adapter conformance

- Clang C/C++: compilation database, SARIF/structured diagnostics, LLVM records, AST/IR identity.
- Cargo/rustc: Cargo compilation units, rustc JSON diagnostics, matching LLVM profile, Rust item identity.
- TypeScript: compiler API diagnostics, program graph, configuration identity, patch interventions.
- GCC C/C++: JSON diagnostics, optimization records, and compiler-divergence observations.
- Require positive, negative, ambiguous, tool-failure, and policy-denied fixtures for every adapter.

### Semantic obligation engine

- Model pointer-loaded bounds, indexed writes, and relevant access extents.
- Preserve the distinction between LLVM assumptions and enforceable source contracts.
- Derive candidate non-overlap conditions for supported patterns.
- Identify negative, overflowed, unbounded, volatile, atomic, and concurrent cases.
- Return typed refusal reasons when derivation is unsound or incomplete.

### Agent workflow

- Give Codex compact compiler evidence and precise repository questions.
- Inspect callers, declarations, FFI boundaries, documentation, and tests.
- Compare `restrict`, guarded versioning, API changes, and refusal.
- Require the agent to explain rejected unsafe alternatives.
- Generate tests for both guarded branches and rerun compiler evidence.

### Product experience

- Keep one-command diagnosis with progressive detail.
- Provide human, JSON, and artifact-directory outputs from one semantic model.
- Make declines specific and actionable.
- Supply a pinned, judge-testable environment and deterministic fixtures.
- Surface provenance without overwhelming the primary result.

### Verification

- Differentially test original and repaired code over generated and adversarial inputs.
- Preserve an explicit overlap case that exercises the unchanged fallback.
- Confirm the selected fast path vectorizes from optimization records, not assembly heuristics alone.
- Benchmark with warmup, repeated samples, environment capture, and uncertainty reporting.
- Test determinism, schema compatibility, and malicious project inputs.

## Cross-phase acceptance gates

No phase may weaken these gates:

1. **Reproducibility:** another machine using the pinned environment can reproduce the result.
2. **Isolation:** the declared assumption is the only intended experimental delta.
3. **Identity:** baseline and variant outcomes refer to the same source loop with adequate confidence.
4. **Semantic honesty:** report language matches evidence strength.
5. **Safe failure:** ambiguity and unsupported constructs produce typed declines.
6. **Agent accountability:** every source change traces to compiler evidence and repository evidence.
7. **Behavior preservation:** covered overlap and non-overlap cases retain expected behavior.
8. **Performance evidence:** any speed claim includes retained measurements and environment metadata.

## Phase documents

- [Phase 00 — Foundation](docs/phases/00-foundation.md)
- [Phase 01 — Baseline analysis](docs/phases/01-baseline-analysis.md)
- [Phase 02 — Counterfactual search](docs/phases/02-counterfactual-search.md)
- [Phase 03 — Obligation derivation](docs/phases/03-obligation-derivation.md)
- [Phase 04 — Agent workflow](docs/phases/04-agent-workflow.md)
- [Phase 05 — Validation and benchmarking](docs/phases/05-validation-benchmarking.md)
- [Phase 06 — Product hardening](docs/phases/06-product-hardening.md)
- [Phase 07 — Expansion](docs/phases/07-expansion.md)

## Plan maintenance

Update phase status only when its entry or exit conditions materially change. Record the validating command and artifact in [logs/validation.md](logs/validation.md). Record design changes in an ADR rather than rewriting historical reasoning.
