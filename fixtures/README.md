# Fixture catalog

Fixtures are executable semantic cases, not illustrative snippets. Each case defines its source, toolchain expectations, analysis classification, agent behavior, and adversarial validation requirements.

## Case classes

- `bound-alias` — baseline misses and a bound-parameter noalias counterfactual vectorizes.
- `already-vectorized` — LLVM runtime versions a conventional input/output alias case; WhyVec must decline further search.
- `refusal` — volatile pointer-loaded bound invalidates ordinary obligation and repair reasoning.

## Fixture rules

- Keep compiler expectations pinned to a toolchain and target profile.
- Retain raw optimization records when expectations are refreshed.
- Never update an expected result merely to make a test pass; record the toolchain change and review the semantic impact.
- Include negative behavior that would expose an unsafe `restrict` or cached-bound repair.
- Link every fixture to the report-schema and phase gates it exercises.

The canonical index is [manifest.json](manifest.json).
