# Model usage log

Record material GPT-5.6 and Codex work that affects product output: task, supplied deterministic evidence, repository scope inspected, decision made, alternatives rejected, files changed, and validation performed.

Do not record API keys, raw hidden reasoning, private prompts, or private repository contents. Model output is never compiler evidence until verified by deterministic tools.

## 2026-07-21T15:25:33Z — R8 repository-action forward tests

Task:

- consume linked optimization, source-obligation, guarded-validation, and
  repository evidence;
- choose among `restrict`, guarded runtime versioning, API change, and refusal;
- retain a source-action trace without upgrading deterministic evidence.

Deterministic evidence supplied:

- public `explain-opt` and `replay-opt` reports for `add_vectors_`;
- public `derive-obligation` and `replay-obligation` reports;
- schema 1.1 validation tied to the exact candidate SHA-256;
- tracked repository occurrences, external-linkage uncertainty, source diff,
  command outcomes, sanitizer/compiler records, and benchmark distributions.

Repository scope inspected:

- the original bound-alias fixture, ABI-preserving candidate, public signature,
  guard arithmetic, branch and production harnesses, benchmark, retained
  reports, action trace, schemas, skill instructions, and agent contract.

Decision:

- reject `restrict` because external callers remain uncertain and LLVM
  parameter `noalias` is broader than the derived loop-range condition;
- reject an API change because the candidate can preserve `void add_vectors_`;
- select guarded runtime versioning only after the exact production candidate
  passed linked ABI, differential, sanitizer, optimization-record, and
  benchmark gates;
- refuse the volatile obligation case and require validation for a mismatched
  candidate digest.

Forward-test corrections:

- an older validation 1.0 bundle and absent upstream reports correctly caused
  refusal;
- the first R8 candidate changed symbol/return type, so it was replaced by an
  ABI-preserving source and revalidated;
- test-only ABI changes and an unrelated `noinline` constraint were removed;
- production mode now passes the full behavior, sanitizer, compiler, and
  benchmark gates, with branch-helper exposure retained only as supplemental
  evidence;
- pruned declared searches now use `smallest_set_found` rather than declared-
  space minimality.

Files and evidence:

- plugin and skill under `integrations/codex/whyvec/`;
- action schema `schemas/whyvec-agent-trace.schema.json`;
- retained bundle `evidence/codex-action/2026-07-21/`;
- workflow validation in `scripts/verify_optimization_causality.py`.

Evidence strength:

- compiler outcomes are observed;
- `parameter.count.noalias` is a tested sufficient assumption in the evaluated
  singleton tier;
- candidate behavior is validated on covered executions, not proved
  equivalent.

## 2026-07-21T16:12:41Z — R3–R8 completion forward audit

Task:

- audit every R3–R8 release claim against current implementation, retained
  evidence, phase gates, architecture, schemas, threat model, risk register,
  executable validation, and remote CI state.

Material findings and decisions:

- remote `master` was not complete while GitHub's moving Ubuntu runner lacked
  usable Bubblewrap namespaces; the job was pinned to Ubuntu 22.04 with an
  explicit full-mode sandbox smoke check, and run `29846353697` passed;
- product and demonstration documents named commands absent from executable
  help, so they were corrected to the implemented explain/replay/observe and
  obligation surfaces rather than expanding claims;
- checked-in action evidence needed fresh manifest verification rather than a
  trusted prior replay boolean;
- guarded selection must reject measured `noise_decline` and require every ABI,
  instrumented, production, sanitizer, compiler, and benchmark command plus
  both branch and overflow witnesses.

Forward-test outcomes:

- exact measured candidate still selects `validated_guarded_runtime`;
- a candidate digest mismatch and missing fast-path coverage select
  `validation_required`;
- completed covered behavior with `noise_decline` selects `refuse` while
  retaining `validated on covered executions`;
- a volatile obligation still selects typed refusal;
- a copied retained bundle with a removed pre-optimization IR artifact is
  rejected by repository validation.

Evidence strength:

- compiler outcomes remain observed;
- LLVM parameter `noalias` remains a tested sufficient assumption only in the
  evaluated singleton tier;
- behavior remains validated on covered executions;
- performance is a measured result only when the retained dispersion rule
  classifies `measured_improvement`.
