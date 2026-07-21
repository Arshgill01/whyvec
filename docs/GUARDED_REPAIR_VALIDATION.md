# Guarded repair validation

The first source-action fixture implements the derived
`bound_object_disjoint_from_modified_region` obligation without adding
`restrict` or changing the fallback's loop condition.

## Repair shape

The guarded implementation:

1. reads the initial bound before assuming disjointness;
2. declines the fast path for zero or negative bounds;
3. computes output extent and both integer address-range ends with checked
   arithmetic;
4. enters the cached-bound fast path only when the bound object is disjoint
   from the complete modified output interval;
5. otherwise executes the original `i < *count` loop unchanged.

The address conversion policy is explicitly limited to the recorded flat
x86-64 `uintptr_t` target. Arithmetic uncertainty selects the fallback.

## Executable validation

`scripts/verify_guarded_repair.py` requires a positive retained obligation
report and runs four gates with the fingerprinted Clang toolchain:

- original-versus-repaired differential executions;
- AddressSanitizer and UndefinedBehaviorSanitizer over the same defined corpus;
- structured optimization records for both branches;
- alternating-order repeated benchmark samples with median and median absolute
  deviation.

The differential corpus covers disjoint storage, the bound immediately before
and after the output range, the bound inside the modified output range,
input/output overlap in both directions, zero and negative bounds, and checked
address overflow. Branch counters demonstrate that both the fast path and the
unchanged fallback execute.

The validator fails before writing a success report if behavior, branch,
sanitizer, or compiler-record gates fail. Performance is classified separately:
an improvement is reported only when median separation exceeds three times the
sum of both median absolute deviations; otherwise the result is
`noise_decline`.

## Retained covered execution

The checked-in [validation report](../evidence/guarded-bound-alias/2026-07-21/report.json)
and its content-digested artifacts record:

- 9 differential executions: 5 fast-path and 4 fallback selections;
- 2 checked-overflow refusals;
- clean ASan/UBSan results on the same covered executions;
- a vectorized fast-path loop and missed unchanged fallback loop;
- 31 raw samples per implementation plus CPU, kernel, affinity, governor, and
  compiler identity;
- a 3.51× median ratio for that retained workload and environment.

This is validated on covered executions. It is not a claim of full semantic
equivalence, portability beyond the recorded target policy, or performance on
another workload or machine.

The historical report above uses validation schema 1.0. The [R8 Codex action
bundle](../evidence/codex-action/2026-07-21/README.md) uses schema 1.1, which
also binds validation to the exact guarded candidate digest and retains exit
status plus stdout/stderr digests for every compile and execution command. Its
separate ABI-preserving candidate adds smallest and largest fixture-supported
bounds, for 11 differential executions: 7 fast paths and 4 fallbacks.

The executable product demo is the expanded current gate. The exact
model-authored candidate runs 3,271 deterministic defined-behavior executions:
1,123 fast paths and 2,148 fallbacks, including every bound position throughout
the modified interval for counts 2–65. ASan/UBSan repeat that corpus. Eleven
unsafe candidate/evidence mutations must be rejected, and the structured YAML
parser must uniquely observe the VF=8/IC=4 fast loop and the missed fallback.
Eight benchmark sizes retain warmup, seeded randomized ordering, 31 paired raw
samples, and dispersion. A noisy run returns `noise_decline`; it never borrows
the retained session's measured result.
