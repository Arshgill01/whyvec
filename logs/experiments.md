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

## 2026-07-21T11:59:26Z — Build-causality semantic replay and tamper check

Commands:

```console
python3 scripts/verify_build_causality.py
whyvec replay-build <generated-report.json>
```

Observed results:

- The generated Cargo fixture retained the exact captured patch/untracked atom
  payloads, raw bounded Cargo JSON and stderr streams, and SHA-256 plus byte
  length for each artifact.
- The report recorded the base commit, aggregate input digest, normalized
  command digest, rustup active toolchain, Cargo and rustc invocation/resolved
  binaries, delegated tools, versions, and binary digests.
- A replay with unchanged repository input and toolchain re-executed the search
  and matched the original normalized semantic digest. Run artifact bytes were
  allowed to differ because ephemeral worktree paths are not compiler
  semantics; diagnostic identity, verdicts, minimality, and causal evidence
  remained identical.
- After one retained artifact was modified, replay stopped at the content
  integrity check and did not claim reproduction.

Evidence strength and limitation:

- This is a reproduced counterfactual observation for the generated fixture on
  the recorded local toolchain. Replay currently depends on the original local
  Git repository and base object; it is not yet a portable redacted
  reproduction bundle.

## 2026-07-21T12:18:41Z — Clang printable-pipeline counterfactual replay

Command:

```console
python3 scripts/verify_compiler_fixtures.py
```

Observed results:

- Clang 21.1.8 emitted its instantiated O3 pass sequence for the pinned
  x86-64-v3 fixture. Matching `opt-21` replay of that sequence observed the same
  uncountable-loop miss at `kernel.c:5` as the monolithic baseline.
- The LLVM API transformer changed only argument 2 to parameter-level
  `noalias`; replay then observed vectorization at width 8 and interleave count
  4. Argument 1 remained a negative singleton, and argument 0 was also a
  successful but broader singleton.
- The monolithic `count_noalias.c` witness independently observed the same
  preferred changed outcome.

Evidence strength and limitation:

- This is a tested sufficient assumption under the recorded Clang/LLVM
  toolchain and an `equivalent_confirmed` pipeline. It is not an `exact`
  pipeline claim, because LLVM's printable pass string is best-effort, and it
  does not establish that the source-level alias contract is true.

## 2026-07-21T12:41:24Z — Public retained Clang optimization query

Command shape:

```console
whyvec explain-opt <fixture>/kernel.c:5 --function add_vectors_ \
  --parameter output:0 --parameter input:1 --parameter count:2 \
  --transformer <pinned-helper> --identity-tool <pinned-helper> --format json
```

Observed results:

- The monolithic and recorded-pipeline baselines both missed the structurally
  matched loop.
- All declared singletons were evaluated. `count noalias` and `output noalias`
  were observed sufficient; `input noalias` was not observed sufficient.
- Both successful singleton outcomes repeated consistently with width 8 and
  interleave count 4. The report therefore states
  `minimal_in_declared_search`, not unique minimality.
- A separate already-vectorized baseline declined before search, and the
  volatile-bound search completed without a successful supported assumption.

Evidence strength and limitation:

- Counterfactual observation under the fingerprinted tools and
  `equivalent_confirmed` pipeline. The explicit source-to-IR mapping is an input
  to this development query. No source obligation or repair authority is
  claimed by this report.

## 2026-07-21T12:54:22Z — Optimization semantic replay and tamper check

Commands:

```console
python3 scripts/verify_optimization_causality.py
whyvec replay-opt <generated-report.json>
```

Observed results:

- An unchanged rerun of the public bound-alias query reproduced the original
  loop identity, baseline and singleton outcomes, confirmation consistency,
  finite-search trace, minimality, and normalized semantic digest.
- Replay verified the source digest and captured Clang 21.1.8, `opt`, typed
  transformer, and loop-identity helper fingerprints before starting the new
  analysis.
- After one declared artifact was made writable and modified, replay stopped
  at SHA-256/size verification and did not execute or claim reproduction.

Evidence strength and limitation:

- This reproduced the counterfactual observation on the recorded local
  toolchain. The report points to the original repository and helper binaries;
  it is not yet a portable or redacted reproduction bundle.

## 2026-07-21T13:03:39Z — Integrated ambiguous-loop refusal

Command shape:

```console
whyvec explain-opt fixtures/cases/ambiguous-loop/kernel.c:2 \
  --function ambiguous --parameter output:0 --parameter input:1 \
  --transformer <pinned-helper> --identity-tool <pinned-helper> --format json
```

Observed results:

- The pre-optimization function contained two distinct loops starting at the
  selected debug line.
- The public query retained both identity-helper streams and returned
  `identity.ambiguous` with two matches.
- The report had no selected subject, split-pipeline baseline, experiments, or
  finding, and labeled pipeline fidelity `not_evaluated`.
- `replay-opt` reproduced the same decline semantic digest.

Evidence strength and limitation:

- This is an evidence-backed refusal. No claim is made about either loop's
  individual optimization outcome because the supplied location did not
  uniquely identify one.

## 2026-07-21T13:09:16Z — Adversarial Cargo build-script containment

Command:

```console
python3 scripts/verify_build_causality.py
```

Observed results:

- The generated base repository included a `build.rs` that attempted a direct
  TCP connection to `1.1.1.1:80`, a write into the original repository, and a
  write under `/tmp`.
- Every build completed inside Bubblewrap. The network connection and original
  repository write were denied, while the build script's successful `/tmp`
  write remained inside the private tmpfs and did not appear on the host.
- The resulting report retained Bubblewrap invocation/resolved binary digests,
  version, and enabled isolation properties; semantic replay matched with the
  same sandbox fingerprint.

Evidence strength and limitation:

- This validates the covered network and mount-isolation attempts on the
  recorded Linux host. It is not a claim of immunity from kernel or compiler
  vulnerabilities, and seccomp/cgroup resource enforcement remains separate.

## 2026-07-21T13:23:50Z — Dependent Rust function-edit grouping

Command:

```console
python3 scripts/verify_build_causality.py
```

Observed results:

- The captured API change contained separate zero-context signature and body
  hunks inside `measure` plus an unrelated hunk inside `stable`.
- Parsed old/new Rust item spans produced two syntax groups: one `measure`
  function group containing both dependent hunks and one `stable` group.
- The `measure` group alone reproduced the selected caller `E0308`. Its two
  invalid partial edits were not represented or executed as independent search
  candidates.
- Removing that group from the full change suppressed the target and its
  recorded cascade; public replay reproduced the same group search and digest.

Evidence strength and limitation:

- This is a tested sufficient syntax-item edit group under the recorded Cargo
  build. Grouping prevents false hunk independence; it does not establish that
  every edit in the function is semantically necessary or correct.

## 2026-07-21T13:30:07Z — C++ linkage, template, and macro experiments

Commands:

```console
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_optimization_causality.py
```

Observed results:

- The C-linkage `add_vectors_cpp` baseline missed; `count noalias` and
  `output noalias` were observed sufficient singletons while `input noalias`
  remained negative under the equivalent-confirmed Clang 21 pipeline.
- The explicitly instantiated `template_add<int>` was selected by
  `_Z12template_addIiEvPT_PKS0_PKi` and produced the same singleton outcomes.
- Two `APPLY` macro expansions on one invocation line produced two identity
  matches. Public `explain-opt` retained `identity.ambiguous`, no subject, and
  no variants.

Evidence strength and limitation:

- These are C++ compiler counterfactual observations. They do not establish
  `__restrict` validity, template-wide contracts, or safe changes at any caller.
