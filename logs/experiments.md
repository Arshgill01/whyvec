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

## 2026-07-21T09:46:42Z — Cargo/rustc build-causality end-to-end query

Fixture topology:

- A passing Git base contains an API accepting `i32` and two consumers.
- The working tree changes that API to accept `&str`, producing two distinct `E0308` diagnostics in different source files.
- An unrelated tracked file changes independently.
- An unrelated untracked file is present.

Commands:

```console
cargo test -p whyvec-build isolates_one_failure_inducing_file_and_confirms_removal -- --nocapture
python3 scripts/verify_build_causality.py
```

Observed results:

- The detached base worktree passed `cargo check`.
- The reconstructed full change emitted the selected `E0308` identity.
- File atoms were enumerated deterministically across tracked and untracked content.
- `src/api.rs` was the unique minimal sufficient file set in the declared search.
- The complement build retained the unrelated changes while removing the target diagnostic.
- A second `E0308` identity disappeared with the target and was retained as a co-suppressed diagnostic.
- The public CLI emitted a schema-valid JSON report and persisted the same report beneath the analyzed repository's `.whyvec/analyses` directory.

Evidence strength:

- Counterfactual observation under the recorded base commit, file atoms, Cargo command, and rustc diagnostic identities.
- This establishes tested build-diagnostic sufficiency and a full-patch removal witness. It does not establish that the API edit is semantically wrong or identify a line-level cause within the file.

## 2026-07-21T09:54:52Z — Stable diagnostic identity and repeatability check

Command:

```console
python3 scripts/verify_build_causality.py
```

Observed results:

- Selecting `E0308` without a path refused because the full candidate contained two matching observations and printed both stable diagnostic identities.
- Selecting `E0308` at `src/lib.rs` completed and retained one exact identity.
- Rerunning with that identity reproduced the same atom list, ordered subset verdicts, minimality, sufficient set, removal witness, and suppressed diagnostic identities.
- Retained `.whyvec/` reports did not enter the repeated search space.
- Both reports validated against the Draft 2020-12 build-causality schema.

This confirms deterministic causal output for the generated fixture's declared inputs. It does not claim reproducibility across unrecorded toolchain or environment changes.

Each future entry must include the analysis identifier, source digest, normalized compilation command, compiler binary digest, target and feature set, selected loop identity, declared assumption delta, artifact paths, isolation result, outcome classification, and exact reproduction command.

## 2026-07-21T11:40:09Z — Nested file-to-hunk build causality

Commands:

```console
cargo test -p whyvec-build --all-features
python3 scripts/verify_build_causality.py
```

Observed results:

- A sufficient Rust source file containing an API-breaking hunk and an unrelated value-change hunk was reconstructed from two captured zero-context patches.
- The API hunk alone reproduced the selected `E0308`; the unrelated hunk built successfully without it.
- Removing only the sufficient hunk from the complete working-tree patch eliminated the target and its co-suppressed diagnostics while retaining the unrelated hunk.
- A separate `E0119` fixture added two conflicting trait implementations in distant hunks. Each singleton built successfully; the two-hunk set was the unique minimal sufficient set.
- Repeated CLI analysis by exact diagnostic identity retained the same nested causal projection and both reports passed schema validation.

Evidence strength: executable counterfactual observation over captured Git hunks. A hunk is an intervention region, not a semantic AST cause.
