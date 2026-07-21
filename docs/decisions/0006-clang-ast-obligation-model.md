# ADR 0006: Derive the first source obligation from Clang's structured AST

## Status

Accepted for the first C bound-alias family.

## Context

The LLVM parameter-level `noalias` intervention is broader than the source
condition needed to keep a pointer-loaded loop bound stable. Translating the
successful intervention directly into `restrict` or a pairwise address check
would overstate the evidence. The engine needs source entities, types,
evaluation shape, induction semantics, and every relevant write before it can
state a candidate obligation.

## Decision

WhyVec runs the fingerprinted Clang from the retained optimization report with
`-ast-dump=json` and applies a versioned `clang_ast_affine_bound_v1` access
model. The model accepts one uniquely selected C `for` loop with:

- a constant integral lower bound;
- a positive constant induction step;
- an `induction < *bound_parameter` condition;
- direct affine indexed writes through function parameters;
- fixed-width scalar layouts under the recorded x86-64 target policy;
- no volatile, atomic, call, or unsupported write semantics.

The report retains the exact AST stream, source and optimization evidence
digests, bound object, write regions, induction domain, checked arithmetic
requirements, untouched-fallback requirement, and typed declines. Semantic
replay verifies all upstream and derived evidence.

## Consequences

- A result is a `derived_obligation`, not a repository-supported contract.
- The LLVM assumption remains separately named in every positive report.
- C++ and Rust require their own source models and decline here.
- Non-affine, volatile, atomic, call-bearing, ambiguous, or layout-unknown cases
  refuse rather than approximating.
- Runtime enforcement is limited to the recorded flat `uintptr_t` target
  policy and must fall back on every arithmetic uncertainty.
