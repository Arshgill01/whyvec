# R3–R8 completion audit

> Status correction (2026-07-21): this historical audit established the
> deterministic R8 handoff, not an actual installed-skill GPT-5.6 Codex
> repository session. [PLAN.md](../PLAN.md) now keeps R8 in `verifying` until
> that executable model evidence is retained.

## Scope

`R3` through `R8` are the release capabilities named in
[PLAN.md](../PLAN.md). The numbered documents under [docs/phases](phases)
provide their detailed semantic, safety, workflow, and validation gates. This
audit maps the release claims to executable evidence; it does not silently mark
the still-active R1/R2 work or the broader future distribution and
counterfactual-family surfaces complete.

The authoritative pass/fail state remains [PLAN.md](../PLAN.md), with exact
commands and remote-run evidence appended to
[logs/validation.md](../logs/validation.md). A document or schema alone is not
completion evidence.

## Release capability matrix

| Release | Required capability | Executable evidence |
| --- | --- | --- |
| R3 | Adapter-neutral deterministic three-valued search, honest minimality, bounded argv-only processes, immutable create-new artifacts, and semantic replay with tamper refusal | `whyvec-experiment` unit tests; `verify_build_causality.py`; `verify_optimization_causality.py` |
| R4 | Typed LLVM parameter intervention, structural isolation, equivalent-confirmed pipeline replay, stable loop identity, successful confirmations, already-vectorized/no-success/ambiguity declines, and retained replay | `verify_compiler_fixtures.py`; `verify_llvm_transformer.py`; `verify_llvm_loop_identity.py`; `verify_optimization_causality.py` |
| R5 | Cargo/rustc diagnostic identity, immutable Git atoms, syntax-item grouping with exact-hunk fallback, interacting changes, removal witnesses, mandatory Bubblewrap containment, public CLI replay, and tamper refusal | Fifteen `whyvec-build` tests; `verify_build_causality.py`; GitHub `Repository integrity` workflow |
| R6 | C++ linkage/template positives and macro ambiguity; Cargo, Clang, GCC, and TypeScript build adapters; GCC native optimization observation and integrity-checked LLVM comparison | `verify_compiler_fixtures.py`; `verify_cross_adapter_build_causality.py`; `verify_optimization_causality.py` |
| R7 | Typed C bound obligation and volatile refusal; checked flat-x86-64 guard; unchanged fallback; branch, overflow, ABI, differential, sanitizer, compiler-record, environment, and dispersion-aware benchmark evidence | Three `whyvec-obligation` tests; `verify_guarded_repair.py`, invoked by `verify_optimization_causality.py`; retained `evidence/guarded-bound-alias/2026-07-21/` and R8 validation 1.1 report |
| R8 (deterministic portion only) | Report replay and compatibility checks, preliminary caller inventory, all four strategy decisions, exact candidate/validation linkage, unsafe-alternative rejection, typed refusal, complete command ledger, action trace, and installable Codex plugin | `plan_action.py`; action-trace schema; positive, digest-mismatch, missing-branch, benchmark-noise, and volatile-refusal forward tests in `verify_optimization_causality.py`; official plugin and skill validators; retained `evidence/codex-action/2026-07-21/`. These do not substitute for the pending actual-model session. |

## Detailed phase-gate mapping

### Experiment isolation and identity

- Accepted LLVM variants are produced by the typed LLVM API transformer and
  differ only by declared parameter attributes.
- Baseline and variant loops are selected by structural LLVM identity. Missing,
  duplicate, macro-origin, and post-delta identity failures decline rather than
  select a nearby loop.
- Search uses a three-valued oracle. Unresolved smaller sets block minimality;
  pruned searches say `smallest_set_found`.
- Build subsets run in fresh detached worktrees through mandatory fingerprinted
  Bubblewrap. Wrapper, response-file, plugin, unstructured-output, network, and
  host-write adversarial cases are refused or contained.

### Obligation, guarded behavior, and measurement

- The obligation report keeps LLVM `parameter.count.noalias` distinct from the
  source bound-object/modified-region predicate.
- Unsupported volatile and atomic dimensions retain typed declines. Checked
  extent/range arithmetic refuses overflow.
- The exact production candidate preserves the public C ABI. Eleven retained
  differential executions are validated on covered executions: seven fast
  paths and four unchanged fallbacks, including overlap. Two synthetic
  address-end overflow cases refuse the fast path.
- Instrumented and production ASan/UBSan commands pass on the covered corpus.
  Clang observed the fast loop vectorized and the fallback loop missed.
- Thirty-one alternating-order samples per implementation, median/MAD,
  environment, affinity, and CPU policy are retained. `noise_decline` now
  selects refusal even when covered behavior remains validated.

### Repository action and refusal

- The planner replays both upstream WhyVec reports and verifies every linked
  artifact manifest before emitting a create-new trace.
- External-linkage uncertainty rejects `restrict`; absent repository contract
  authority rejects an API change; a guarded candidate requires its exact
  digest, all thirteen successful command outcomes, both branch witnesses,
  clean sanitizer coverage, compiler records, overflow refusal, and measured
  improvement.
- A mismatched candidate or incomplete branch ledger returns
  `validation_required`. A declined obligation or completed benchmark
  `noise_decline` returns `refuse`.
- The checked-in action trace retains inspected references, uncertainty,
  strategy comparison, normalized diff, commands, outcomes, evidence language,
  and residual risks. It is an audit handoff, not proof of caller closure.

## Cross-phase invariant evidence

| Invariant | Retained or executable evidence |
| --- | --- |
| Reproducibility | Source/tool/command digests, raw compiler records, normalized semantic digests, public replay commands, repeated-query comparisons, and pinned version profiles |
| Isolation | Fresh worktrees, read-only host root, private `/tmp`, typed LLVM structural diff, and unchanged compiler pipeline/target inputs |
| Identity | Stable diagnostic fingerprints and LLVM structural loop fingerprints with explicit ambiguity declines |
| Semantic honesty | Evidence-strength types, schema enums, claim-language checks, `smallest_set_found`, candidate-obligation separation, and covered-execution wording |
| Safe failure | Three-valued unresolved results; stable baseline, identity, search, obligation, validation, and policy refusals; artifact-tamper tests |
| Agent accountability | Replayed report identifiers/digests, repository inventory, four-strategy comparison, exact candidate digest/diff, thirteen command outcomes, and residual risks |
| Behavior preservation | Original-versus-candidate differential corpus with explicit fast/fallback witnesses and production plus instrumented sanitizer executions |
| Performance evidence | Raw alternating samples, compiler/environment identity, median/MAD dispersion rule, measured-improvement classification, and deterministic noise refusal |

## Audit corrections

The completion audit found and corrected four claims that earlier narrow checks
did not catch:

1. GitHub's moving Ubuntu runner lacked or blocked the mandatory Bubblewrap
   namespace setup. CI now names Ubuntu 22.04, installs Bubblewrap, and runs the
   complete sandbox smoke command before tests.
2. Product and demonstration documents advertised CLI commands that did not
   exist. They now list the executable explain, replay, observe, and obligation
   commands and label future convenience surfaces as such.
3. Repository validation rehashed only the guarded-validation artifacts while
   trusting prior replay booleans for the R8 bundle. It now verifies the
   optimization, obligation, validation, and replay-analysis manifests and
   refuses missing, escaped, symlinked, size-mismatched, or digest-mismatched
   evidence.
4. The action planner accepted `noise_decline` and incomplete branch evidence.
   It now requires the complete thirteen-command ledger, both branch witnesses,
   sanitizer coverage equal to the differential corpus, overflow refusal, and
   `measured_improvement`; covered behavior remains labeled accurately when
   performance causes refusal.

## Explicit residual scope

- Rust split-pipeline optimization evidence remains `surrogate` and cannot
  authorize a Rust source action.
- The golden C path now ingests one real compilation-database entry and maps
  direct C parameters automatically. C++ ABI mapping and ambiguous entries
  remain explicit declines or expert overrides.
- Build replay currently requires its recorded local Git object and toolchain;
  portable redacted export is a later distribution gate.
- The first runtime guard is limited to the recorded flat x86-64 `uintptr_t`
  policy.
- Tracked-text caller discovery is preliminary. External, indirect, dynamic,
  generated, and FFI edges remain uncertain and therefore do not justify
  `restrict`.
- Differential and sanitizer results are validated on covered executions, not
  full semantic equivalence. Benchmark classification applies only to the
  retained machine and workload.
