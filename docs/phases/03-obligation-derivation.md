# Phase 03: Obligation derivation and refusal model

## Entry conditions

- A confirmed sufficient counterfactual exists.
- Source and IR entities involved in the assumption map with adequate confidence.
- Access-analysis inputs are available.

## Deliverables

- Typed access summaries for pointer-loaded bounds and loop writes.
- Affine induction and write-extent analysis.
- Candidate bound-object versus modified-region non-overlap obligation.
- Checked arithmetic model for runtime-enforceable extents.
- Explicit preconditions and unsupported semantics.
- Stable obligation decline taxonomy.
- Human and JSON obligation rendering.
- Queries exposing the evidence behind each derived range.

## Edge cases

- Signed negative bounds, zero trips, non-unit steps, and overflow.
- Multiple exits, early returns, calls, and exceptions.
- Volatile, atomic, concurrent, signal-visible, and reentrant mutation.
- Non-affine indices and pointer chasing.
- Multiple stores with distinct bases and extents.
- Custom address spaces and non-integral pointers.
- Bound object wider or narrower than the induction type.
- Undefined baseline inputs and sanitizer findings.

## Exit gates

- Supported fixtures derive the exact source entities and predicates expected.
- Every unsupported semantic dimension produces a typed decline.
- Parameter `noalias` and source obligation remain visibly distinct in all outputs.
- Overflow and provenance tests cannot route uncertain inputs into the fast path.
- Obligation renderings contain sufficient data for independent review.
