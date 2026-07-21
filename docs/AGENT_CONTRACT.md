# Codex and GPT-5.6 contract

## Role

The agent translates deterministic counterfactual evidence into a repository-aware engineering decision. It does not infer compiler outcomes from prose and does not strengthen contracts merely to obtain a green optimization remark.

## Required workflow

1. Run WhyVec against the user-selected source location.
2. Read the machine report and confirm it is complete, supported, and free of confounds.
3. Inspect the selected function, declarations, all resolvable callers, FFI boundaries, documentation, tests, and build configuration.
4. Identify whether the candidate obligation is already guaranteed, enforceable at runtime, enforceable through an API change, or unsupported.
5. Compare repair strategies explicitly.
6. Explain why unsafe alternatives were rejected.
7. Apply the narrowest repository-consistent repair.
8. Add tests that distinguish the original semantics from an unjustified hoist or alias annotation.
9. Run build, behavior, overlap, fallback, compiler, sanitizer, and benchmark checks required by the report.
10. Rerun WhyVec on the repaired source and retain the before/after evidence.
11. Summarize what is observed, tested, assumed, and still uncertain.

## Repair strategy hierarchy

### Existing contract enforcement

Use a source annotation only when repository evidence already establishes its full language semantics for every caller. Evidence may include public API documentation, type systems, language/FFI contracts, and audited call sites. Tests alone are insufficient.

### Guarded versioning

Use a runtime guard when the condition can be computed without assuming it and an unchanged fallback can preserve original behavior. The guard must dominate every optimized-path assumption.

### API correction

Change an API when the intended contract is real but not expressible or enforceable in the current signature. Update declarations, callers, tests, documentation, and compatibility notes together.

### Refusal

Refuse the repair when:

- caller coverage is incomplete;
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
- commands and outcomes for every required check;
- before/after optimization records;
- benchmark raw data and summary method;
- residual risks and unsupported callers.

## Context discipline

Give GPT-5.6 the finding, candidate obligation, exact relevant source spans, caller summaries, and required checks. Keep full IR and raw optimization records available through targeted queries rather than dumping them into context.

The agent must request more deterministic evidence when a decision depends on compiler facts absent from the report.
