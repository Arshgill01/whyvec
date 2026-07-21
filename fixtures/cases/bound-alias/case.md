# Bound-alias counterfactual

## Expected baseline

Under the pinned Clang 21 x86-64-v3 profile, the loop misses vectorization because the loop trip count is not established as stable.

## Expected search

- Modeling `input` as parameter-level `noalias` does not resolve the unstable trip count.
- Modeling `count` as parameter-level `noalias` is sufficient for the selected loop to vectorize.
- Modeling `output` as parameter-level `noalias` may also be sufficient but imposes a broader source contract.

The preferred explanatory finding uses the narrower relevant subject, `count`, while reporting every successful evaluated singleton.

## Required obligation behavior

The obligation engine identifies the object loaded through `count` and the region modified through `output`. It must keep LLVM's parameter assumption distinct from the candidate source-level non-overlap condition.

## Adversarial behavior

A validation harness must include a case where `count` points into `output`. An unconditional cached-bound rewrite changes that program's behavior and must fail the regression. A guarded fast path must select the untouched original loop for this layout.
