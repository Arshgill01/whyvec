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

## 2026-07-21T15:45:57Z — R8 Codex repository-action exit validation

Passed commands:

```console
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo fmt --all -- --check
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo test --workspace --all-targets --all-features
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/validate_repository.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_compiler_fixtures.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_build_causality.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_cross_adapter_build_causality.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_llvm_transformer.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_llvm_loop_identity.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_optimization_causality.py
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py integrations/codex/whyvec/skills/whyvec-optimize
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py integrations/codex/whyvec
```

Clean-checkout artifact validation used commit `aea37cd` at
`/home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-clean-FkiWVT/repo` and
copied its plugin to the sibling `installed-whyvec` directory. The following
all passed against that copied artifact:

```console
python3 scripts/validate_repository.py
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py installed-whyvec/skills/whyvec-optimize
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py installed-whyvec
python3 -m py_compile installed-whyvec/skills/whyvec-optimize/scripts/plan_action.py
```

Results:

- All forty-one Rust tests, formatting, Clippy, repository checks, compiler
  fixtures, build adapters, LLVM helpers, and optimization workflow passed.
- Validation schema 1.1 retains thirteen command outcomes for the exact
  candidate digest. Eleven covered executions agreed with the original: seven
  selected the checked fast path, four selected the unchanged fallback, and
  two synthetic address-end overflow cases refused the fast path.
- ABI, production and instrumented differential tests, production and
  instrumented ASan/UBSan tests, branch-specific optimization records, and the
  production benchmark all completed successfully. The compiler observed the
  fast loop vectorized and the fallback loop missed.
- The retained action trace selected `validated_guarded_runtime`. It rejected
  `restrict` because repository caller coverage is incomplete, rejected an API
  change as unnecessary, and preserves typed refusal for unsupported
  obligations or mismatched candidate evidence.
- Optimization and obligation reports replayed with matching semantic digests.
  The retained search result is described as the `smallest_set_found`, because
  the declared finite search was not exhaustively evaluated.
- A model forward audit found no unresolved material defect after fixes for
  legacy-report authorization, ABI mismatch, test-only evidence, and minimality
  overstatement.
- The benchmark is evidence for this covered x86-64 environment, not a portable
  performance guarantee. Differential and sanitizer tests establish only
  `validated on covered executions`, not full semantic equivalence.

## 2026-07-21T15:51:15Z — GitHub sandbox dependency repair validation

The current `master` workflow failure was inspected with authenticated `gh`:
run `29845598565`, job `88685272487`. Repository validation, formatting, and
Clippy passed; only the two build-causality tests that require Bubblewrap
failed because the runner image did not provide `bwrap`.

Passed commands after adding the explicit workflow installation step:

```console
bwrap --version
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo test -p whyvec-build --all-features
python3 scripts/validate_repository.py
git diff --check
```

Results:

- Bubblewrap 0.11.1 was available to the local reproduction.
- All fifteen build-adapter tests passed, including the two isolated causal
  analyses that failed remotely.
- Repository validation and patch whitespace validation passed.
- Remote completion is not claimed until the pushed workflow rerun succeeds.

## 2026-07-21T15:56:32Z — Pinned-runner Bubblewrap smoke validation

Passed commands for the revised workflow setup and focused regression:

```console
bwrap --die-with-parent --new-session --unshare-all --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp -- /usr/bin/true
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo test -p whyvec-build --all-features
python3 scripts/validate_repository.py
git diff --check
```

Results:

- The same mount, process, cgroup, and network namespace mode used by the build
  adapter started successfully.
- All fifteen build-adapter tests passed.
- Product documentation now lists only implemented CLI commands; planned
  progressive-detail commands are explicitly labeled as future hardening.
- The next GitHub run must pass on the named Ubuntu 22.04 runner before this
  evidence closes the remote distribution gate.

## 2026-07-21T16:18:12Z — R3–R8 requirement-by-requirement completion audit

The audit used [the retained gate matrix](../docs/R3_R8_COMPLETION_AUDIT.md) to
check the PLAN release claims against the numbered phase gates, architecture,
semantic and agent contracts, schemas, threat model, risk register, current
sources, adversarial fixtures, retained artifacts, executable help, and GitHub
state. It did not infer completion from the pre-existing `complete` labels.

GitHub run `29846353697` for commit `e2c5d8e` passed on the named Ubuntu 22.04
runner. Its Bubblewrap install and full `--unshare-all` smoke, repository
validation, formatting, Clippy, and all Rust tests succeeded.

After the gate-audit corrections were committed as `6cbcac0`, a fresh local
clone was created at
`/home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-final-audit-6Kaovk/repo`.
The plugin was copied to the sibling `installed-whyvec` directory. These exact
commands passed from that clean checkout:

```console
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo fmt --all -- --check
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp cargo test --workspace --all-targets --all-features
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/validate_repository.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_compiler_fixtures.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_build_causality.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_cross_adapter_build_causality.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_llvm_transformer.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_llvm_loop_identity.py
TMPDIR=/home/arshdeepsingh/work/whyvec-validation-tmp python3 scripts/verify_optimization_causality.py
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py /home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-final-audit-6Kaovk/installed-whyvec/skills/whyvec-optimize
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py /home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-final-audit-6Kaovk/installed-whyvec
python3 -m py_compile /home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-final-audit-6Kaovk/installed-whyvec/skills/whyvec-optimize/scripts/plan_action.py
git status --short
```

Results:

- All forty-one Rust tests, formatting, and Clippy passed from an empty build
  cache. The clean clone remained unmodified.
- Clang 21/C++, rustc 1.96.1/LLVM 22 surrogate, Cargo/rustc, Clang SARIF,
  GCC 15 JSON/native optimization records, and TypeScript 7 fixture paths all
  produced their expected positive or typed-refusal results.
- Public build, optimization, GCC, and obligation replay matched normalized
  semantics; modified reports or artifacts remained refused.
- R8 forward tests retained guarded selection for the exact measured candidate,
  returned `validation_required` for digest mismatch and missing fast-branch
  coverage, and returned `refuse` for volatile obligations and benchmark
  `noise_decline` without erasing covered behavior evidence.
- Repository validation independently rehashed the optimization, obligation,
  validation, and replay-analysis manifests in the checked-in action bundle.
  A copied-bundle test with `baseline/preopt.ll` removed was refused with the
  exact missing-artifact path.
- The copied plugin passed the official skill and plugin validators, and its
  planner byte-compiled successfully.

Residual scope remains explicit: the Rust split pipeline is surrogate,
automatic source/IR mapping is still R1/R2 work, portable redacted build export
is a later distribution gate, the guard is limited to the recorded flat x86-64
policy, caller discovery is not closed-world proof, tests validate covered
executions only, and benchmark evidence is environment-specific.

## 2026-07-21T18:21:50Z — Compilation-database and structured-record product checkpoint

Passed commands:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/validate_repository.py
python3 scripts/verify_optimization_causality.py
ctest --test-dir demo/build --output-on-failure
python3 scripts/verify_guarded_repair.py --obligation-report <retained-positive-report> --artifact-root <fresh-directory>
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py integrations/codex/whyvec
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py integrations/codex/whyvec/skills/whyvec-optimize
```

Results:

- Forty-seven Rust tests passed. Compilation-database discovery preserved the
  semantic argv, expanded and fingerprinted bounded response files, and
  declined missing, ambiguous, wrapper, shell, plugin, and escaping inputs.
- `whyvec analyze demo/src/kernel.c:4` selected the real CMake/Ninja entry,
  inferred `add_vectors_` and its pointer parameters, observed the baseline
  miss, found `parameter.count.noalias` as a tested sufficient assumption,
  derived the candidate obligation, and emitted a schema-valid agent packet.
- LLVM YAML records, not human stderr, determined baseline and variant
  classifications. Malformed, duplicate, missing, unrelated, ambiguous, and
  version-varying record cases passed.
- The exact compilation configuration now drives monolithic IR, replay, and
  Clang AST obligation extraction. Replay re-resolves and compares the
  compilation entry and response-file fingerprints.
- Guarded validation schema 1.2 carried a versioned required-check plan and
  structured fast/fallback records; the full optimization workflow passed.
- The repo marketplace installed WhyVec through `codex plugin add` at plugin
  version `0.1.0+codex.20260721181611`. Actual-model exit evidence remains the
  next R8 gate and is not claimed by this checkpoint.

## 2026-07-21T19:07:21Z — Productization and adversarial validation checkpoint

Passed commands:

```console
python3 scripts/verify_demo_mutations.py
cargo test -p whyvec-obligation
python3 scripts/verify_real_world_superlu.py --whyvec target/debug/whyvec --output evidence/real-world/superlu-a9314310
python3 scripts/check_portable_evidence.py
python3 scripts/validate_repository.py
./scripts/demo --ci
python3 -m jsonschema -i evidence/codex-live/2026-07-21/validation-report.json schemas/whyvec-validation-report.schema.json
python3 -m jsonschema -i evidence/codex-live/2026-07-21/action-trace.json schemas/whyvec-agent-trace.schema.json
```

Results:

- The exact retained model candidate ran 3,271 defined-behavior differential
  executions: 1,123 fast paths and 2,148 unchanged fallbacks. ASan/UBSan covered
  the same corpus, and structured YAML observed fast VF=8/IC=4 plus the missed
  fallback.
- All eleven unsafe mutations were rejected. The eight-size benchmark used
  seven warmups, seeded paired ordering, and 31 samples per size; the repeated
  demo classified a measured improvement with a 5.16x representative median
  ratio on this machine.
- Eight obligation tests passed, including stable declines for inclusive and
  descending induction, nested loops, early exits, calls, non-affine indexes,
  unknown extents, volatile/atomic access, and ambiguous locations. Positive
  tests cover unsigned wide bounds, non-unit steps, and multiple writes.
- The pinned SuperLU commit `a93143107e3854ba9716ee3d7ab40fca6880cc10`
  configured with its real CMake database, built `blas` and `s_test`, passed
  repository test `s_test_9_2_0_LA`, observed the SAXPY cleanup-loop miss, found
  no successful assumption, retained a principled refusal, and replayed with a
  matching semantic digest.
- The actual installed-skill session and both portable evidence families passed
  schema, digest-link, and private-path/reasoning redaction checks.

## 2026-07-21T19:15:54Z — Final local product checkpoint rerun

Passed commands after the schema and induction-update corrections:

```console
python3 -m py_compile scripts/*.py demo/tools/*.py
bash -n containers/judge/build.sh
python3 scripts/validate_repository.py
python3 scripts/check_portable_evidence.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 scripts/verify_compiler_fixtures.py
python3 scripts/verify_build_causality.py
python3 scripts/verify_cross_adapter_build_causality.py
python3 scripts/verify_llvm_transformer.py
python3 scripts/verify_llvm_loop_identity.py
python3 scripts/verify_optimization_causality.py
python3 scripts/verify_demo_mutations.py
./scripts/demo --ci
python3 scripts/verify_real_world_superlu.py --whyvec target/debug/whyvec --output <fresh-directory>
python3 /home/arshdeepsingh/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py integrations/codex/whyvec
python3 /home/arshdeepsingh/.codex/skills/.system/skill-creator/scripts/quick_validate.py integrations/codex/whyvec/skills/whyvec-optimize
cargo install --path crates/whyvec-cli --root <fresh-directory>/install
python3 scripts/build_helpers.py --output <fresh-directory>/install/bin
<fresh-directory>/install/bin/whyvec doctor --format json
```

The clean install reported `ready: true` with Clang/LLVM 21, both bundled
helpers, CMake, Ninja, Rust, Python, and Codex resolved. The independent
SuperLU checkout again passed its repository-native test and retained a
principled refusal. The pinned judge image cannot be built locally because no
Docker daemon is available; the dedicated GitHub job is the distribution gate.
