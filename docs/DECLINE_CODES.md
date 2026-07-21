# Decline code registry

Declines are stable product outcomes, not generic errors. A decline states which stage stopped, why the available evidence was insufficient, which artifacts were retained, and what new evidence could make another attempt meaningful.

Minor releases may add codes. Removing a code or changing its meaning requires a report-schema major version.

## Input

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `input.source_outside_root` | Canonical source path escapes the allowed repository root. | Select a source inside the allowed root or change policy explicitly. |
| `input.compile_command_missing` | No compilation entry maps to the selected source. | Generate a compilation database or supply a complete command. |
| `input.compile_command_ambiguous` | Multiple semantically distinct entries remain. | Select the intended command identifier. |
| `input.location_not_loop` | The requested source location cannot be mapped to a loop. | Select an exact loop line or stable loop identifier. |
| `input.response_file_unreadable` | A required response file cannot be resolved under policy. | Provide a readable, in-root response file. |

## Policy

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `policy.shell_execution_denied` | The command requires shell interpretation. | Supply a tokenized direct compiler invocation. |
| `policy.wrapper_denied` | An unapproved compiler wrapper is present. | Approve and fingerprint the wrapper or bypass it. |
| `policy.compiler_plugin_denied` | The command loads an unapproved plugin. | Remove it or authorize its digest explicitly. |
| `policy.path_escape` | An input, output, symlink, or response file crosses an allowed boundary. | Resolve the path inside the analysis policy. |
| `policy.network_required` | Reproduction attempts network access under deny policy. | Materialize dependencies locally and rerun offline. |
| `policy.resource_limit` | Process, memory, output, or duration bounds were exceeded. | Reduce the case or raise one named policy limit. |

## Toolchain

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `toolchain.unsupported_compiler` | The selected frontend is outside the supported Clang family. | Use a supported profile or add a validated integration. |
| `toolchain.version_mismatch` | Binary version does not match the selected profile. | Select the matching profile or toolchain. |
| `toolchain.digest_mismatch` | A pinned binary digest differs. | Re-establish the trusted distribution digest. |
| `toolchain.pipeline_unreplayable` | The recorded optimization pipeline cannot reproduce the frontend outcome. | Capture a replayable pipeline or analyze monolithically. |
| `toolchain.target_incomplete` | Target, CPU, features, SDK, or sysroot is missing. | Supply the complete target fingerprint. |

## Baseline

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `baseline.compile_failed` | The unmodified command fails. | Repair or reproduce the baseline build first. |
| `baseline.already_vectorized` | The selected loop already vectorizes. | Investigate profitability or runtime-versioning questions outside this family. |
| `baseline.loop_absent` | The selected loop is removed before the measured decision. | Select the earlier decision point or a surviving loop. |
| `baseline.outcome_inconsistent` | Confirmation runs produce different semantic outcomes. | Remove nondeterministic inputs and stabilize the build. |
| `baseline.no_missed_record` | A scalar outcome lacks a correlatable structured record. | Enable compatible optimization records or improve mapping evidence. |

## Loop identity

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `identity.ambiguous` | More than one loop matches the source and structural signals. | Select a stronger identifier or simplify the fixture. |
| `identity.low_confidence` | Too few independent signals correlate baseline and variant. | Retain debug columns, pre-opt IR, and structural metadata. |
| `identity.transformed_beyond_match` | A variant transforms the loop beyond reliable correspondence. | Move the observation point or add family-specific matching. |

## Variant

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `variant.invalid_delta` | The requested assumption cannot be applied to the selected IR entity. | Correct the source-to-IR mapping. |
| `variant.isolation_failed` | Structural diff contains changes outside the declared delta. | Regenerate from the immutable baseline and inspect the transformer. |
| `variant.verifier_failed` | Modified IR is rejected by LLVM verification. | Correct the typed transformer; never optimize invalid IR. |
| `variant.optimizer_failed` | The optimizer exits unsuccessfully. | Retain the crash bundle and reproduce under the pinned binary. |
| `variant.timed_out` | A bounded variant run exceeded policy. | Reduce the case or adjust the named execution bound. |
| `variant.outcome_inconsistent` | Confirmation runs disagree. | Remove unstable inputs or decline the finding. |

## Search

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `search.no_successful_assumption` | Every evaluated assumption set still misses. | Add a semantically modeled family; do not ask the model to guess. |
| `search.incomplete` | Resource or strategy limits stop before the declared space is covered. | Resume remaining subsets or use weaker minimality language. |
| `search.loop_lost` | The target cannot be matched in one or more required variants. | Improve identity evidence before comparing outcomes. |

## Obligation

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `obligation.access_not_affine` | Modified regions cannot be bounded by the supported affine model. | Add a sound family-specific access analysis. |
| `obligation.extent_unknown` | One or more relevant access extents remain unknown. | Establish explicit bounds for every relevant access. |
| `obligation.arithmetic_overflow` | Guard range arithmetic cannot be represented safely. | Supply a checked representation or stricter input contract. |
| `obligation.volatile_bound` | The bound is deliberately re-read as volatile. | Provide domain semantics that justify a distinct transformation. |
| `obligation.atomic_or_concurrent` | Atomic or concurrent mutation escapes the sequential model. | Supply a memory-model-aware analysis. |
| `obligation.observable_call` | A call may mutate or observe relevant state inside the loop. | Establish effect summaries for the call. |
| `obligation.non_integral_pointer` | The target address space does not support the proposed range guard. | Provide a target-specific enforcement model. |
| `obligation.undefined_baseline` | The analyzed execution relies on undefined behavior. | Define the source behavior before optimizing it. |

## Verification

| Code | Meaning | Required next evidence |
| --- | --- | --- |
| `verification.repository_contract_missing` | Caller inspection does not establish a global promise. | Use runtime enforcement or produce authoritative contract evidence. |
| `verification.external_callers_unknown` | Public, indirect, FFI, or dynamic callers cannot be audited. | Preserve compatibility and avoid a global annotation. |
| `verification.overlap_case_missing` | No test exercises the original alias-sensitive behavior. | Add an intentional overlap regression case. |
| `verification.fallback_not_observed` | Tests do not demonstrate fallback routing. | Instrument or otherwise establish branch execution. |
| `verification.behavior_mismatch` | Original and repaired executions disagree. | Reject the repair and retain the failing corpus. |
| `verification.fast_path_not_vectorized` | The repaired fast path remains scalar. | Reanalyze the actual compiled path before making a speed claim. |
| `verification.benchmark_inconclusive` | Noise or guard cost prevents a credible benefit. | Improve measurement or retain the original implementation. |

## Contributing a code

A new code requires:

- one stable meaning and owning stage;
- one fixture or unit test that emits it;
- retained evidence fields appropriate to that stage;
- safe next-action language;
- report and human-renderer compatibility tests;
- an update to this registry and any schema enum that constrains it.
