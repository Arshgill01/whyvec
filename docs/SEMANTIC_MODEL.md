# Semantic model

## Evidence lattice

WhyVec attaches an evidence strength to every material statement.

From weakest to strongest:

1. `compiler_message` — text emitted by the compiler.
2. `compiler_record` — structured optimization record tied to a source location.
3. `counterfactual_observation` — a changed outcome under an isolated declared delta.
4. `derived_obligation` — a source condition produced by a supported access model.
5. `repository_supported_contract` — callers, types, documentation, or tests establish the condition within their stated scope.
6. `runtime_enforced_contract` — executable control flow checks the condition and retains a behavior-preserving fallback.
7. `formally_verified_property` — a named proof tool validates a precisely stated property and emits a retained artifact.

No stage may silently emit language belonging to a stronger stage.

## Counterfactual claim

For baseline inputs `I`, optimization pipeline `P`, and assumption delta `A`:

```text
P(I) = missed
P(apply(I, A)) = vectorized
```

WhyVec may conclude:

> Under the recorded toolchain and pipeline, `A` was sufficient to change the matched loop's observed vectorization outcome.

It may not conclude that `A` is true, necessary, globally safe, uniquely causal, portable, or sufficient for performance improvement.

## Search minimality

Let `S` be the declared finite candidate set and `E` the evaluated subsets.

- `smallest_set_found` means no evaluated successful set has lower cardinality.
- `minimal_in_declared_search` requires every subset smaller than the reported set to be evaluated and unsuccessful.
- `unique_minimal_in_declared_search` additionally requires every same-cardinality alternative to be evaluated and unsuccessful.

The report includes `S`, `E`, stop conditions, and skipped subsets.

## LLVM parameter `noalias`

Parameter-level `noalias` constrains memory locations accessed through pointer values based on the argument when those locations are modified during the function execution. It is intentionally similar to C `restrict` semantics.

Consequences:

- It is not merely a statement that two starting addresses differ.
- It is not inherently limited to the selected loop.
- It may cover accesses not represented by a simple `[base, base + length)` range.
- Violating it can make optimized behavior undefined.
- Adding it in shadow IR is valid for experimentation but not authorization to add `restrict` to source.

## Bound-alias obligation

For the supported shape:

```c
for (i = L; i < *bound; i += step)
    output[f(i)] = ...;
```

the compiler may be unable to establish a stable trip count if writes through `output` can modify the object read through `bound`.

A derived obligation must name:

- the bound object and byte extent read;
- every loop write region that could overlap it;
- the iteration domain used to compute those regions;
- signedness, negative-bound, overflow, and zero-trip handling;
- whether the condition is required globally or only for a guarded fast path.

Example conceptual obligation:

```text
modified_bytes(output, iterations(initial(*bound)))
  ∩ bytes(bound, sizeof(*bound))
  = ∅
```

This notation is not by itself executable. Runtime enforcement needs target-aware address-range logic with overflow handling and a preserved fallback.

## Guarded versioning semantics

A safe guarded repair has this abstract form:

```text
capture values needed to evaluate the guard without assuming the guard
if enforceable_condition:
    execute a fast path whose assumptions are dominated by the guard
else:
    execute the original loop without hoisting or strengthening assumptions
```

Review requirements:

- Capturing the initial bound must match the first observable evaluation of the original loop.
- Guard evaluation must not dereference memory the original would not dereference.
- Address arithmetic must not introduce language-level undefined behavior.
- The fallback must not use the cached bound if the original could observe bound mutation.
- Concurrent mutation, volatile objects, atomics, signals, and reentrant callbacks invalidate ordinary guard reasoning unless explicitly modeled.
- The fast path must not execute for negative or overflowed extents that the range calculation cannot represent.

## Validation semantics

- Compilation validates syntax and compiler acceptance.
- Optimization records validate the compiler's reported transformation.
- Tests validate covered executions.
- Differential tests increase behavioral confidence over generated inputs.
- Sanitizers detect selected classes of runtime errors.
- Benchmarks measure the recorded environment and workload.
- Formal tools validate only their modeled property and bounds.

Reports and documentation must state these scopes exactly.
