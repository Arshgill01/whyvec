---
name: whyvec-optimize
description: Diagnose and repair Clang missed-vectorization cases using WhyVec counterfactual reports. Use when the user asks why a C loop stayed scalar, provides a WhyVec JSON report, wants Codex to inspect callers before applying restrict or a runtime guard, or needs correctness and benchmark validation for a compiler-guided optimization.
---

# WhyVec optimize

Use deterministic WhyVec output as compiler evidence, inspect the repository for the missing contract, and select a repair only when the evidence supports it. Never turn a successful shadow compilation into an assertion that real callers satisfy the tested assumption.

## Required inputs

Before changing source, obtain:

- the WhyVec report or the command and source location needed to produce it;
- the exact baseline compilation entry and toolchain fingerprint;
- the selected loop identity and counterfactual finding;
- the repository's applicable instructions and validation commands.

If the report is unavailable or fails its schema, run diagnosis before proposing a repair. If the WhyVec executable is unavailable, explain that compiler causality remains unverified and do not simulate its output.

## Workflow

### 1. Read the evidence without upgrading its strength

Read [references/report-reading.md](references/report-reading.md). Confirm that baseline and variant refer to the same loop, their fingerprints match outside the declared delta, and the outcome actually changed. Distinguish:

- observed compiler outcome;
- tested sufficient assumption;
- candidate source obligation;
- repository contract established from callers and tests.

Stop if experiment isolation, loop identity, or report provenance is ambiguous.

### 2. Establish the real repository contract

Trace every reachable declaration and caller that can affect the pointer relationship. Inspect:

- public headers and API documentation;
- wrappers, callbacks, function pointers, FFI, and dynamic boundaries;
- allocation and slicing behavior;
- tests that intentionally exercise overlap;
- build modes and target-specific implementations.

Absence of an overlapping caller is not evidence of a non-overlap contract. Prefer a documented invariant, type-level invariant, checked precondition, or explicit runtime enforcement.

### 3. Select or refuse the repair

Read [references/repair-policy.md](references/repair-policy.md). Evaluate in this order:

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

### 5. Validate causality, correctness, and value

Run repository-native tests plus the report's verification plan. At minimum:

- confirm the original baseline remains reproducible;
- confirm the chosen fast path vectorizes from optimization records;
- exercise non-overlap through the fast path;
- exercise intentional overlap through the fallback;
- test zero, negative where representable, extreme, and overflow-adjacent bounds;
- repeat identical analysis and compare normalized report output;
- benchmark only after correctness checks pass.

Do not claim full semantic equivalence from tests. Report the executions and properties covered.

### 6. Return an evidence ledger

Summarize:

- what Clang observed;
- which assumption changed that outcome;
- what repository evidence established or failed to establish;
- which repair was selected and which alternatives were rejected;
- exact validation commands and outcomes;
- remaining semantic and performance risks.

Link the source patch, tests, report artifact, and benchmark artifact when available.

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
