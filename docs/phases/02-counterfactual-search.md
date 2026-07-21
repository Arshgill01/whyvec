# Phase 02: Counterfactual noalias search

## Entry conditions

- Baseline loop identity is stable.
- Pre-optimization IR is verified and immutable.
- Optimization pipeline replay matches the baseline result.

## Deliverables

- Typed parameter-level `noalias` assumption family.
- Source-to-IR parameter mapping.
- Stable candidate enumeration and exclusion reasons.
- Singleton and configured subset search.
- Exact structural-delta verification before optimization.
- Variant execution and artifact capture.
- Loop outcome comparison with confidence.
- Minimality classification based on evaluated subsets.
- Confirmation runs for successful counterfactuals.

## Edge cases

- Existing `noalias`, `byval`, `sret`, address spaces, and pointer attributes.
- Opaque pointers and compiler-generated parameters.
- Inlining or argument specialization before the measured pass.
- Multiple successful singletons.
- Pairwise success without singleton success.
- Loop deletion or transformation before vectorization.
- Variant verifier failure, optimizer crash, timeout, or nondeterminism.
- Search resource bounds and skipped combinations.

## Exit gates

- Structural diff proves every accepted variant changed only declared attributes.
- Search report accurately distinguishes found, minimal, unique, and incomplete results.
- Successful fixtures reproduce the baseline miss and variant vectorization.
- No-success fixtures return a typed conclusion without model speculation.
- Confounded loop matching declines rather than selecting a nearby loop.
- Every experiment is reproducible from retained artifacts.
