# Experiment log

## 2026-07-21T08:29:16Z — Cross-frontend bound-alias mechanism check

Toolchains:

- Clang and LLVM `21.1.8`, target `x86-64-v3`.
- rustc `1.96.1` using LLVM `22.1.2`; external LLVM `22.1.2`, target `x86-64-v3`.

Command:

```console
python3 scripts/verify_compiler_fixtures.py
```

Observed results:

- The real Clang baseline emitted `Cannot vectorize uncountable loop` for the C bound-alias fixture.
- Replaying the Clang pre-optimization IR through the paired `default<O3>` pipeline remained scalar.
- Adding only parameter-level `noalias` to the `count` IR argument changed the matched loop to vectorized; adding it only to `input` did not. `output` also produced a successful, broader singleton.
- The real rustc optimized baseline emitted the same uncountable-loop analysis for the Rust raw-pointer fixture.
- The paired LLVM 22 split baseline remained scalar. The same singleton outcome set was observed for `count`, `input`, and `output` interventions.

Evidence strength and limitation:

- Clang fixture: observation-level equivalence confirmed between the monolithic baseline and paired split pipeline; this is not a claim that every pipeline detail is identical.
- Rust fixture: counterfactual observation in a matching-version `default<O3>` surrogate. It demonstrates frontend-neutral IR feasibility but cannot authorize a Rust source repair until exact replay or pipeline equivalence is established.

Artifacts were generated in an ephemeral isolated directory by the verifier and were not retained. Product experiment records must retain immutable artifacts once the experiment runtime owns this flow.

Each future entry must include the analysis identifier, source digest, normalized compilation command, compiler binary digest, target and feature set, selected loop identity, declared assumption delta, artifact paths, isolation result, outcome classification, and exact reproduction command.
