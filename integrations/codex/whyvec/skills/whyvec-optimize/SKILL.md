---
name: whyvec-optimize
description: Diagnose, repair, or explicitly refuse Clang missed-vectorization work using WhyVec optimization, source-obligation, validation, and repository-action reports. Use when the user asks why a C loop stayed scalar, provides WhyVec JSON evidence, wants Codex to inspect callers before applying restrict or a runtime guard, or needs a correctness and benchmark validation trace for a compiler-guided optimization.
---

# WhyVec optimize

Use deterministic WhyVec output as compiler evidence, inspect the repository for the missing contract, and select a repair only when the evidence supports it. Never turn a successful shadow compilation into an assertion that real callers satisfy the tested assumption.

## Required inputs

Before changing source, obtain:

- the WhyVec optimization report or the command and source location needed to produce it;
- the linked WhyVec source-obligation report;
- linked validation evidence when a guarded candidate already exists;
- the exact baseline compilation entry and toolchain fingerprint;
- the selected loop identity and counterfactual finding;
- the repository's applicable instructions and validation commands.

If a report is unavailable, incompatible, fails replay, or has an invalid artifact manifest, run the relevant WhyVec command before proposing a repair. If the WhyVec executable is unavailable, explain that compiler causality remains unverified and do not simulate its output.

## Workflow

### 1. Read the evidence without upgrading its strength

Read [references/report-reading.md](references/report-reading.md). Replay the optimization and obligation reports with `whyvec replay-opt` and `whyvec replay-obligation`. Confirm that baseline and variant refer to the same loop, their fingerprints match outside the declared delta, and the outcome actually changed. Distinguish:

- observed compiler outcome;
- tested sufficient assumption;
- candidate source obligation;
- repository contract established from callers and tests.

Stop if experiment isolation, loop identity, or report provenance is ambiguous.

Resolve the planner relative to the directory containing this `SKILL.md`; do not assume the user's repository root contains `scripts/plan_action.py`. Run it before editing:

```bash
python3 <skill-directory>/scripts/plan_action.py \
  --optimization-report <optimization-report.json> \
  --obligation-report <obligation-report.json> \
  --validation-report <validation-report.json> \
  --whyvec <whyvec-binary> \
  --repository <repository-root> \
  --candidate-source <candidate-source> \
  --output <new-action-trace.json>
```

Omit `--validation-report` and `--candidate-source` when no guarded candidate exists. Run the planner only when the user has authorized a source change, because report replay retains new analyses. Pass a create-new `--output` path. For an answer-only review, inspect existing retained reports and traces without invoking this mutating workflow. Read [references/action-trace.md](references/action-trace.md) before using its result.

### 2. Establish the real repository contract

Treat `repository_discovery` from the action trace as a preliminary tracked-text inventory, not a proof of a closed caller set. Trace every reachable declaration and caller that can affect the pointer relationship. Inspect:

- public headers and API documentation;
- wrappers, callbacks, function pointers, FFI, and dynamic boundaries;
- allocation and slicing behavior;
- tests that intentionally exercise overlap;
- build modes and target-specific implementations.

Use language-aware repository search and build metadata to find edges that text search cannot establish. Absence of an overlapping caller is not evidence of a non-overlap contract. Prefer a documented invariant, type-level invariant, checked precondition, or explicit runtime enforcement.

### 3. Select or refuse the repair

Read [references/repair-policy.md](references/repair-policy.md). Compare all four alternatives retained by the action trace, then evaluate in this order:

1. retain the existing implementation if the benefit is unsupported or immaterial;
2. strengthen an already-established source contract when all callers satisfy it;
3. add a runtime-guarded fast path with the original loop as fallback;
4. redesign the API only when the repository already supports that scope;
5. refuse when the obligation cannot be established or enforced safely.

Never add `restrict`, `llvm.assume`, alias metadata, vectorization pragmas, or unsafe intrinsics merely because they make the compiler report change.

### 4. Implement the smallest justified patch

Preserve evaluation order, loop-bound behavior, integer semantics, zero-trip behavior, volatility, atomics, and externally observable effects. Evaluate a range guard without pointer-order comparisons that are undefined or unrelated-object pointer subtraction. Guard byte-size calculations against overflow.

When runtime versioning is selected:

- evaluate the guard before caching or hoisting the pointer-loaded invariant;
- keep the original loop structurally intact in the fallback;
- ensure the fast path carries only the contract established by the guard;
- avoid making the fallback unreachable through optimizer assumptions.

Do not copy a candidate patch blindly. Apply the smallest repository-consistent version, then update the action trace or retain a new one that identifies the actual candidate digest and diff.

### 5. Validate causality, correctness, and value

Run repository-native tests plus the report's verification plan. At minimum:

- confirm the original baseline remains reproducible;
- confirm the chosen fast path vectorizes from optimization records;
- exercise non-overlap through the fast path;
- exercise intentional overlap through the fallback;
- test zero, negative where representable, extreme, and overflow-adjacent bounds;
- repeat identical analysis and compare normalized report output;
- benchmark only after correctness checks pass.

When using the bundled guarded bound-alias fixture, run `python3 scripts/verify_guarded_repair.py` with the linked obligation report and a fresh artifact directory. For another repository, translate the same branch, sanitizer, compiler-record, and measurement gates into repository-native commands.

Do not claim full semantic equivalence from tests. Report the executions and properties covered.

### 6. Return an evidence ledger

Validate the final action trace against `schemas/whyvec-agent-trace.schema.json` when working in a WhyVec checkout. Summarize:

- what Clang observed;
- which assumption changed that outcome;
- what repository evidence established or failed to establish;
- which repair was selected and which alternatives were rejected;
- exact validation commands and outcomes;
- remaining semantic and performance risks.

Link the source patch, tests, optimization and obligation reports, action trace, and benchmark artifact when available. Use **observed**, **tested sufficient assumption**, and **validated on covered executions** exactly as defined by WhyVec. Do not substitute stronger language.

## Refusal conditions

Refuse an automatic repair when any of these remain unresolved:

- baseline or variant compilation fails;
- optimization flags, target, compiler, pipeline, or source digest drift;
- loop identity is ambiguous after transformation;
- the assumption search is confounded by unrelated IR changes;
- access extents or the pointer-loaded object cannot be bounded safely;
- volatile, atomic, concurrent, signal-visible, or device-memory behavior matters;
- integer overflow makes the proposed range guard unreliable;
- a `restrict` promise is broader than the established caller contract;
- tests cannot cover the overlap fallback;
- the performance claim cannot be reproduced.

A precise refusal is a successful outcome. Preserve the report and name the evidence required to revisit it.
