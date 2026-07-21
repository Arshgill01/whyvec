# Fixture catalog

Fixtures are executable semantic cases, not illustrative snippets. Each case defines its source, toolchain expectations, analysis classification, agent behavior, and adversarial validation requirements.

## Case classes

- `bound-alias` — baseline misses and a bound-parameter noalias counterfactual vectorizes.
- `rust-bound-alias` — rustc emits the same missed decision for a raw-pointer FFI-shaped loop; the paired LLVM surrogate establishes cross-frontend experiment feasibility without authorizing a Rust repair.
- `already-vectorized` — LLVM runtime versions a conventional input/output alias case; WhyVec must decline further search.
- `ambiguous-loop` — two loops share one function/debug line; the integrated query must retain an identity decline without selecting either loop.
- `refusal` — volatile pointer-loaded bound invalidates ordinary obligation and repair reasoning.

## Fixture rules

- Keep every case pinned to its own frontend, compiler, optimizer, target, and pipeline-fidelity profile.
- Retain raw optimization records when expectations are refreshed.
- Never update an expected result merely to make a test pass; record the toolchain change and review the semantic impact.
- Include negative behavior that would expose an unsafe `restrict` or cached-bound repair.
- Link every fixture to the report-schema and phase gates it exercises.

The canonical index is [manifest.json](manifest.json).

Run the executable cross-frontend check with:

```console
python3 scripts/verify_compiler_fixtures.py
```
