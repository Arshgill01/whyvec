# WhyVec execution plan

This is the living implementation plan. Phase documents contain detailed acceptance gates; this file records ordering, dependencies, and current state.

## Status vocabulary

- `queued` — dependency gates are not yet satisfied.
- `ready` — dependency gates are satisfied and work may begin.
- `active` — implementation or validation is underway.
- `verifying` — implementation exists and exit gates are being evaluated.
- `complete` — every exit gate has evidence.
- `blocked` — a named external or semantic condition prevents progress.

## Current state

| Phase | Capability | Status | Evidence |
| --- | --- | --- | --- |
| 00 | Repository foundation and contracts | complete | [Foundation validation](logs/validation.md) |
| 01 | Reproducible Clang baseline analysis | ready | Pending implementation evidence |
| 02 | Counterfactual noalias search | queued | Depends on stable loop identity and baseline capture |
| 03 | Obligation derivation and refusal model | queued | Depends on counterfactual report contract |
| 04 | Codex/GPT-5.6 repository workflow | queued | Depends on stable JSON and refusal semantics |
| 05 | Guarded repair validation and benchmarking | queued | Depends on obligation and agent workflows |
| 06 | Product hardening and distribution | queued | Depends on end-to-end evidence |
| 07 | Counterfactual family expansion | queued | Depends on hardened experiment infrastructure |

## Workstreams

### Deterministic engine

- Discover and normalize compilation database entries without shell interpretation.
- Fingerprint compiler inputs and retain raw optimization records.
- Establish stable loop identity across IR variants.
- Generate one-delta shadow variants.
- Search declared assumption sets deterministically.
- Detect confounded experiments and decline ambiguous comparisons.

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
