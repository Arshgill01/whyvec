# Validation log

Validation entries are appended after commands actually complete. Each entry records UTC time, tool versions, exact command, result, coverage, and retained artifacts where applicable.

## 2026-07-21T04:20:43Z — Foundation validation

Environment:

- Rust `1.96.1`; Cargo `1.96.1`.
- Clang and LLVM `21.1.8` on `x86_64-unknown-linux-gnu` with `x86-64-v3` fixture target.
- Python `3` with Draft 2020-12 `jsonschema` validation available.

Passed commands:

```console
python3 scripts/validate_repository.py
python3 scripts/verify_clang_fixtures.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py integrations/codex/whyvec
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py integrations/codex/whyvec/skills/whyvec-optimize
```

Results:

- Repository paths, local links, JSON parsing, fixture selectors, plugin metadata, skill frontmatter, and text-file invariants passed.
- All three schemas passed Draft 2020-12 schema validation; the fixture manifest validated as an instance of its schema.
- The bound-alias and volatile-bound fixtures remained scalar; the conventional transform fixture vectorized at width 8 and interleave count 4.
- Four domain-model unit tests passed; formatting and strict Clippy checks passed.
- The official Codex plugin and skill validators passed.

## 2026-07-21T08:33:24Z — Causal compiler re-foundation validation

Environment:

- Rust `1.96.1`; Cargo `1.96.1`.
- Clang/LLVM `21.1.8` for the C fixture profile.
- rustc `1.96.1` with LLVM `22.1.2`, plus external LLVM `22.1.2`, for the Rust fixture profile.

Passed commands:

```console
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 -c 'import json; from pathlib import Path; import jsonschema; root=Path("."); schema=json.loads((root/"schemas/fixture-manifest.schema.json").read_text()); data=json.loads((root/"fixtures/manifest.json").read_text()); jsonschema.Draft202012Validator.check_schema(schema); jsonschema.validate(data, schema)'
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Results:

- Repository contracts and the version 2 cross-frontend fixture manifest passed validation.
- Clang baseline miss, monolithic `restrict` witness, split-pipeline baseline, and every declared singleton outcome passed.
- The Rust monolithic baseline and paired LLVM surrogate produced the declared outcomes; the result remains blocked from source-action evaluation by its `surrogate` fidelity.
- Eleven shared-domain and experiment-search tests passed, including three-valued oracle, pipeline-fidelity, stable ordering, interacting sufficient sets, unresolved subsets, and resource-bound gates.
- Formatting and strict Clippy checks passed.

## 2026-07-21T09:46:42Z — Build-causality product validation

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 -c 'import json; from pathlib import Path; import jsonschema; schema=json.loads(Path("schemas/whyvec-build-report.schema.json").read_text()); jsonschema.Draft202012Validator.check_schema(schema)'
```

Results:

- Sixteen Rust tests passed across the domain, experiment search, process runner, rustc diagnostic identity, Git/Cargo build oracle, and causal report path.
- The public `whyvec explain-build` CLI passed against a generated multi-file Cargo repository.
- The report validated against the Draft 2020-12 build-causality schema.
- Process output bounds and process-group timeout behavior passed.
- Existing Clang and rustc/LLVM optimization fixtures remained unchanged and passing.
- Repository validation, formatting, and strict Clippy checks passed.

## 2026-07-21T09:54:52Z — Build-causality hardening validation

This entry supersedes only the earlier build-causality test-count statement; the earlier commands and results remain historical evidence.

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 -c 'import json; from pathlib import Path; import jsonschema; schema=json.loads(Path("schemas/whyvec-build-report.schema.json").read_text()); jsonschema.Draft202012Validator.check_schema(schema)'
git diff --check
```

Results:

- Nineteen Rust tests passed: eight build-causality tests, six domain tests, and five experiment-search tests.
- Strict Clippy and formatting passed without warning suppression for the new adapter paths.
- Untracked atoms were verified immutable after source mutation.
- Non-JSON Cargo message formats were rejected and supported JSON variants were accepted.
- The CLI ambiguity refusal exposed stable identities; an exact-identity rerun produced the same causal projection and excluded retained `.whyvec/` state.
- The real temporary-repository reports validated against the Draft 2020-12 schema.
- Existing Clang and rustc/LLVM fixture verification and repository validation passed.

## 2026-07-21T11:40:09Z — Hunk-refinement validation

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 -c 'import json; from pathlib import Path; import jsonschema; schema=json.loads(Path("schemas/whyvec-build-report.schema.json").read_text()); jsonschema.Draft202012Validator.check_schema(schema)'
git diff --check
```

Results:

- Twenty Rust tests passed: nine build-causality tests, six domain tests, and five shared search tests.
- Singleton and interacting hunk sufficient sets, hunk-level full-patch removal, immutable untracked atoms, process bounds, and diagnostic identity passed.
- Invalid independent patch combinations remain unresolved rather than becoming negative evidence.
- The public CLI report and repeated exact-identity result passed Draft 2020-12 schema validation.
- Existing Clang and rustc/LLVM fixture results remained passing.
- Correction: the invalid-independent-patch path is implemented as `unresolved/intervention_invalid`, but a dedicated context-conflict fixture remains required before treating that path as separately validated.

## 2026-07-21T11:59:26Z — Content-digested build replay validation

Environment:

- Rust `1.96.1`; Cargo `1.96.1`; rustc LLVM `22.1.2` on
  `x86_64-unknown-linux-gnu`.

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 -c 'import json; from pathlib import Path; import jsonschema; schema=json.loads(Path("schemas/whyvec-build-report.schema.json").read_text()); jsonschema.Draft202012Validator.check_schema(schema)'
git diff --check
```

Results:

- Twenty-one Rust tests passed, including rejection of Cargo-named wrapper
  paths in addition to the existing search, identity, process, and isolation
  coverage.
- The generated public-CLI fixture retained SHA-256-addressed atom snapshots,
  normalized command and input digests, Cargo/rustc proxy and delegated-tool
  fingerprints, and bounded stdout/stderr for every compiler run.
- `whyvec replay-build` verified the retained artifact set, recaptured the same
  input and toolchain, reran the complete file/hunk search, and reproduced the
  normalized semantic digest.
- An adversarial byte append to a retained artifact was rejected before replay.
- The generated report passed its expanded Draft 2020-12 schema. Repository,
  Clang, and rustc/LLVM fixture validation remained passing.

## 2026-07-21T12:04:51Z — Shared immutable artifact runtime validation

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/verify_build_causality.py
```

Results:

- Twenty-five Rust tests passed after moving artifact retention out of the
  Cargo adapter and into the adapter-neutral experiment crate.
- Shared tests cover traversal refusal, non-overwriting create-new writes,
  SHA-256/size verification, mutation detection, and read-only finalization.
- The build adapter consumes the shared artifact contract without changing its
  public report schema or replay behavior; the public generated-repository
  replay and tamper-refusal validation remained passing.

## 2026-07-21T12:09:06Z — Shared bounded process runtime validation

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/verify_build_causality.py
```

Results:

- Twenty-six Rust tests passed after moving subprocess execution from the
  Cargo adapter into the shared experiment runtime.
- Shared process tests exercised concurrent bounded output draining,
  process-group timeout termination, and the clear-by-default environment
  allowlist contract.
- The Cargo/Git adapter retained its wrapper refusal, offline build,
  diagnostic identity, file/hunk search, artifact, and replay behavior while
  using the shared runner.

## 2026-07-21T12:13:49Z — Typed LLVM intervention validation

Environment:

- Clang/LLVM and LLVM libraries `21.1.8`.

Passed command:

```console
python3 scripts/verify_llvm_transformer.py
```

Results:

- The LLVM C++ API transformer parsed the pinned bound-alias pre-optimization
  module, selected `add_vectors_` argument 2, applied only parameter-level
  `noalias`, passed LLVM verification, and emitted bitcode accepted by
  `opt-21 -passes=verify`.
- Canonical disassembly comparison established that the fixture output differed
  only by the declared argument attribute (excluding the incidental module-ID
  line).
- Existing `noalias`, non-pointer arguments, and absent functions produced
  typed declines and no accepted variant output.

## 2026-07-21T12:18:41Z — Recorded Clang pipeline replay validation

Passed command:

```console
python3 scripts/verify_compiler_fixtures.py
```

Results:

- The Clang fixture captured the instantiated O3 pass sequence through
  `-mllvm -print-pipeline-passes` and replayed that exact retained string
  through matching `opt-21`, replacing the earlier generic `default<O3>`
  approximation for the C fixture.
- The replay reproduced the observed uncountable-loop baseline miss.
- Variants were produced by the typed LLVM transformer; `count noalias` and
  `output noalias` vectorized while `input noalias` remained scalar.
- The committed monolithic `restrict` witness and the preferred typed replay
  both vectorized. Fidelity remains `equivalent_confirmed`, because LLVM's
  printable pipeline is documented as best-effort rather than an exact
  serialization of every extension callback.

## 2026-07-21T12:23:22Z — LLVM loop identity validation

Passed command:

```console
python3 scripts/verify_llvm_loop_identity.py
```

Results:

- LLVM dominator and loop analysis uniquely matched `add_vectors_` at the
  selected debug line and produced a high-confidence structural fingerprint.
- The same fingerprint, source location, loop depth, and block count were
  observed after the typed parameter-level `noalias` intervention.
- A missing debug line returned `identity.loop_absent`.
- An adversarial IR fixture containing two distinct loops at the same function
  and debug line returned `identity.loop_ambiguous` with two matches; neither
  loop was selected by proximity.

## 2026-07-21T12:41:24Z — Retained optimization-causality CLI validation

Passed commands:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/verify_optimization_causality.py
```

Results:

- Twenty-nine Rust tests passed, including optimization request validation,
  assumption identity, and vector-factor parsing.
- The public Rust `explain-opt` query observed the monolithic and replay
  baseline misses, matched the same pre-optimization loop, evaluated all three
  typed singleton assumptions, and retained the non-unique
  `minimal_in_declared_search` result.
- `count noalias` and `output noalias` consistently vectorized in two runs each
  at width 8/interleave 4; `input noalias` remained a negative singleton.
- The already-vectorized fixture returned `baseline.already_vectorized` without
  running variants. The volatile-bound fixture exhausted its singleton space
  and returned `search.no_successful_assumption` without a finding.
- Tool binary digests/versions, source and pipeline digests, pre/optimized IR,
  structured YAML optimization records, mutation JSON, raw streams, and every
  artifact size/digest were retained read-only. Positive and decline reports
  validated against the Draft 2020-12 development schema.

## 2026-07-21T12:54:22Z — Optimization replay validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 scripts/verify_llvm_transformer.py
python3 scripts/verify_llvm_loop_identity.py
python3 scripts/verify_optimization_causality.py
```

Results:

- All twenty-nine Rust tests, repository validation, cross-frontend compiler
  fixtures, build replay, LLVM transformer, loop identity, and public
  optimization query checks passed.
- The optimization schema accepted positive, already-vectorized, and
  no-success reports with the new repository, replay-limit, and semantic-digest
  fields.
- `replay-opt` reproduced the positive report's semantic digest. Deliberate
  mutation of a retained artifact was refused by its digest/size check.

## 2026-07-21T13:03:39Z — Integrated identity-decline validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 scripts/verify_llvm_transformer.py
python3 scripts/verify_llvm_loop_identity.py
python3 scripts/verify_optimization_causality.py
```

Results:

- All twenty-nine Rust tests and every repository/compiler integration script
  passed with the new manifest-backed ambiguous-loop fixture.
- The direct helper still reported two matches without selecting one; public
  `explain-opt` converted that evidence to a schema-valid retained
  `identity.ambiguous` decline.
- The ambiguity report omitted subject, replay baseline, variants, and finding,
  used `not_evaluated` pipeline fidelity, and reproduced through `replay-opt`.
- Positive, already-vectorized, ambiguous, and no-success reports all validated
  against the updated Draft 2020-12 schema.

## 2026-07-21T13:09:16Z — Cargo Bubblewrap containment validation

Passed commands:

```console
cargo test -p whyvec-build --all-features
python3 scripts/verify_build_causality.py
```

Results:

- Eight build-adapter unit/integration tests passed under mandatory Bubblewrap.
- The public CLI and semantic replay passed with the sandbox identity in the
  build report schema and command digest.
- An adversarial `build.rs` could not reach an external TCP endpoint or write
  into the original repository. Its successful private `/tmp` write did not
  reach the host filesystem.

## 2026-07-21T13:23:50Z — R5 syntax grouping and exit-gate validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 scripts/verify_llvm_transformer.py
python3 scripts/verify_llvm_loop_identity.py
python3 scripts/verify_optimization_causality.py
```

Results:

- Thirty-one Rust tests passed, including parsed multi-hunk function grouping
  and malformed-Rust text fallback coverage.
- The public build query reported two Rust item groups from three exact hunks,
  found the one sufficient function group, retained its two member hunks, and
  reproduced that semantic projection.
- Build and optimization schemas, security containment, tamper refusal,
  compiler fixtures, LLVM identity/delta isolation, and positive/refusal paths
  all remained passing.

## 2026-07-21T13:30:07Z — C++ adapter fixture validation

Passed commands:

```console
python3 scripts/validate_repository.py
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_optimization_causality.py
```

Results:

- The manifest validated with explicit language identities for all eight
  fixtures.
- C++ C-linkage and template baselines plus typed singleton counterfactuals
  matched their expected outcomes; both reports validated against the
  optimization schema and retained `text/x-c++` source evidence.
- The C++ macro case returned a retained `identity.ambiguous` report without a
  selected loop or variant execution.

## 2026-07-21T13:49:45Z — TypeScript and GCC build-adapter validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_build_causality.py
python3 scripts/verify_cross_adapter_build_causality.py
```

Results:

- All thirty-four Rust tests passed, including native GCC JSON, Clang SARIF,
  and TypeScript compiler-API diagnostic parsing.
- The public TypeScript 7 query opened one pinned `tsconfig.json`, observed
  `TS2345`, found `src/api.ts` as the tested sufficient edit, retained the
  removal witness, and reproduced its semantic digest.
- The public GCC query observed `-fpermissive` through GCC's native JSON,
  found `src/api.hpp` as the tested sufficient edit, retained the removal
  witness, and reproduced its semantic digest.
- Both adapters selected the same observation by full stable diagnostic ID,
  used explicit text-hunk fallback, and validated against the shared build
  report schema.
- The existing Cargo/rustc ambiguity, syntax-grouping, hostile-build,
  artifact-tamper, and replay checks remained passing after generic toolchain
  provenance replaced the Cargo-specific report shape.

## 2026-07-21T14:05:18Z — R6 cross-frontend and GCC observation exit validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_cross_adapter_build_causality.py
python3 scripts/verify_optimization_causality.py
```

Results:

- Thirty-eight Rust tests passed. Direct compiler paths, response files,
  plugin-loading options, and non-structured diagnostic formats are refused by
  the native build adapters.
- Cargo/rustc, Clang 21 SARIF, GCC 15 JSON, and TypeScript 7 compiler-API build
  queries selected stable diagnostics by code and full identity, retained
  tested sufficient edit sets and removal witnesses, validated their schema,
  and replayed semantically.
- `observe-gcc-opt` classified the native GCC 15 record for `add_vectors_` as
  missed, mapped the record's process-local pass IDs to stable names, and
  retained compressed and decompressed records.
- The integrity-checked LLVM comparison reported `agrees` for the same
  canonical subject. `replay-gcc-opt` matched the semantic digest, and a
  deliberately modified GCC record was refused.

## 2026-07-21T14:20:15Z — First obligation-family validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_optimization_causality.py
```

Results:

- Forty-one Rust tests passed, including checked extent/range overflow,
  fixed-layout, volatile, and atomic obligation-domain tests.
- The public positive optimization report produced a schema-valid
  `derived_obligation` naming the four-byte `count` object, the indexed writes
  through `output`, the zero-based unit-step iteration domain, checked
  arithmetic, dominating guard, and untouched fallback requirements.
- The report kept `parameter.count.noalias` separate from the candidate source
  range predicate and did not claim a repository-supported contract.
- The volatile-bound report produced `obligation.volatile_bound` and no
  obligation. Both positive and refusal reports retained Clang AST evidence;
  positive semantic replay matched, and modified evidence was refused.

## 2026-07-21T14:33:19Z — R7 guarded source-action exit validation

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_optimization_causality.py
```

Results:

- All forty-one Rust tests and the complete optimization/obligation/repair
  integration passed.
- Nine original-versus-repaired differential executions agreed byte for byte:
  five selected the checked fast path and four selected the unchanged fallback.
  Two synthetic address-end overflow cases refused the fast path.
- The same defined corpus passed AddressSanitizer and
  UndefinedBehaviorSanitizer. The compiler record observed the cached-bound
  fast loop vectorized at width 8/interleave 4 and the original fallback missed.
- Thirty-one alternating-order raw samples per implementation, Clang identity,
  CPU/kernel/affinity/governor, and median/MAD statistics are retained under
  `evidence/guarded-bound-alias/2026-07-21/`.
- On that covered workload and environment, the original median was 4,572,106
  ns and the guarded median was 1,303,592 ns (3.51× median ratio); separation
  exceeded three times the summed MAD under the declared decision rule.
