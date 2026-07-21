# Codex and GPT-5.6 contract

## Role

The agent translates deterministic counterfactual evidence into a repository-aware engineering decision. It does not infer compiler outcomes from prose and does not strengthen contracts merely to obtain a green optimization remark.

## Required workflow

For a failing agent-authored working tree, run `whyvec explain-build` before
editing symptoms individually. Use its sufficient edit sets, unresolved subsets,
and removal witness to identify the semantic change that needs review. Do not
describe a sufficient edit set as inherently incorrect.

1. Run WhyVec against the user-selected source location.
2. Read the machine report and confirm it is complete, supported, and free of confounds.
3. Replay the optimization and obligation reports, then run the bundled action planner to retain the evidence link, preliminary repository inventory, strategy comparison, and candidate digest.
4. Inspect the selected function, declarations, all resolvable callers, FFI boundaries, documentation, tests, and build configuration. Never treat the planner's tracked-text inventory as closed-world proof.
5. Identify whether the candidate obligation is already guaranteed, enforceable at runtime, enforceable through an API change, or unsupported.
6. Compare repair strategies explicitly.
7. Explain why unsafe alternatives were rejected.
8. Apply the narrowest repository-consistent repair.
9. Add tests that distinguish the original semantics from an unjustified hoist or alias annotation.
10. Run build, behavior, overlap, fallback, compiler, sanitizer, and benchmark checks required by the report.
11. Rerun WhyVec on the repaired source and retain the before/after evidence.
12. Validate and retain the action trace, then summarize what is observed, tested, assumed, and still uncertain.

## Repair strategy hierarchy

### Existing contract enforcement

Use a source annotation only when repository evidence already establishes its full language semantics for every caller. Evidence may include public API documentation, type systems, language/FFI contracts, and audited call sites. Tests alone are insufficient.

### Guarded versioning

Use a runtime guard when the condition can be computed without assuming it and an unchanged fallback can preserve original behavior. The guard must dominate every optimized-path assumption.

### API correction

Change an API when the intended contract is real but not expressible or enforceable in the current signature. Update declarations, callers, tests, documentation, and compatibility notes together.

### Refusal

Refuse an annotation or API-contract repair when caller coverage is incomplete. A runtime guard may remain eligible when it enforces the complete derived condition before the optimized path and preserves the original fallback. Refuse every repair when:

- the obligation is broader than repository evidence;
- runtime enforcement is undefined, racy, or not representable;
- the fallback cannot preserve the original behavior;
- the measured loop is not performance-relevant;
- correctness or sanitizer validation fails;
- the optimization disappears under the real build;
- the performance distribution does not justify added complexity.

## Prohibited agent actions

- Add `restrict` because a noalias shadow variant vectorized.
- Cache a pointer-loaded bound unconditionally when the output may alias it.
- Add `#pragma clang loop vectorize(enable)`, `ivdep`, `llvm.assume`, alias metadata, or intrinsics to override compiler safety without a verified contract.
- Rewrite the fallback using the optimized assumptions.
- Hide failing overlap cases from tests or benchmarks.
- Describe differential testing as formal equivalence.
- Report a speedup without the raw benchmark artifact and environment.
- Modify generated or vendored code without locating its source of generation.

## Evidence the agent must retain

- WhyVec report identifier and artifact path;
- files and callers inspected;
- repair alternatives and rejection reasons;
- source diff;
- exact candidate source digest and linked validation-report digest;
- commands and outcomes for every required check;
- before/after optimization records;
- benchmark raw data and summary method;
- residual risks and unsupported callers.

## Context discipline

Give GPT-5.6 the finding, candidate obligation, exact relevant source spans, caller summaries, and required checks. Keep full IR and raw optimization records available through targeted queries rather than dumping them into context.

The agent must request more deterministic evidence when a decision depends on compiler facts absent from the report.

## Executed conformance record

The conformance gate is not satisfied by `plan_action.py` alone. The retained
2026-07-21 session used the installed `$whyvec-optimize` skill in a fresh Codex
CLI 0.144.3 / `gpt-5.6-sol` task. The model inspected the public header, FFI
wrapper, implementation, and overlap test; rejected global `restrict` and
unconditional caching; authored the guard; ran the required checks; and emitted
a schema-constrained ledger. The observable record, prompt, full patch, and
linked deterministic reports are under `evidence/codex-live/2026-07-21/`.
