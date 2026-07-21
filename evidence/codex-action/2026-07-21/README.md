# R8 Codex repository-action evidence

This bundle retains the positive guarded-repair action selected by the
`whyvec-optimize` workflow on 2026-07-21 UTC.

```console
env WHYVEC_RETAIN_AGENT_BUNDLE_ROOT=<repository>/evidence/codex-action/2026-07-21 \
  python3 scripts/verify_optimization_causality.py
```

The action trace selected `validated_guarded_runtime`, rejected `restrict`
because external linkage leaves caller coverage incomplete and LLVM parameter
`noalias` is broader than the derived loop-range condition, and rejected an
API change because no repository contract or compatibility authority was
supplied. The exact production candidate preserves the public
`void add_vectors_` symbol, keeps its guard helper internal, and adds no
unrelated `noinline` constraint.

The bundle contains:

- `action/trace.json`: schema-valid action trace, ABI-preserving candidate diff
  and digest, repository inventory, strategy comparison, commands, outcomes,
  and risks;
- `repository/.whyvec/analyses/`: the immutable original optimization and
  obligation reports plus their replay reports and compiler artifacts;
- `repository/`: the public fixture source snapshot used by those reports;
- `tools/`: the exact x86-64 LLVM helper executables fingerprinted by the
  optimization report;
- `validation/`: schema 1.1 guarded behavior, ABI, sanitizer, compiler-record,
  and benchmark evidence linked to the exact candidate SHA-256;
- `replay.json`: successful public optimization and obligation replay results
  after temporary nested Git metadata was removed.

Clang observed the original loop missed vectorization. The
`parameter.count.noalias` shadow was a tested sufficient assumption in the
evaluated singleton tier; because the larger declared subsets were not run,
the report uses `smallest_set_found`. The candidate is validated on covered
executions: eleven differential executions, seven fast paths, four fallback
paths, smallest and largest fixture-supported bounds, two overflow refusals,
clean recorded ASan/UBSan execution, and a successful default-ABI compile and
execution. Production-mode builds without test instrumentation also pass the
complete differential corpus and ASan/UBSan, emit a vectorized fast loop and
retained fallback miss, and supply the benchmark samples.

The retained benchmark classified a measured improvement on the recorded AMD
EPYC 7B12 environment: original median 4,657,577 ns, guarded median 1,371,561
ns, ratio 3.40, with median separation above three times summed MAD. This is
not a claim for other targets or workloads.

Limitations:

- behavior is validated on covered executions, not proved equivalent;
- the address guard uses the recorded flat x86-64 `uintptr_t` policy;
- the retained helper executables are Linux x86-64 artifacts; another platform
  must rebuild them and produce a new fingerprinted analysis rather than claim
  replay of this toolchain;
- the preliminary tracked-text inventory does not prove a closed caller set;
- retained reports contain local absolute execution paths, while the action
  trace uses bundle-relative report paths. Reproduce on another checkout by
  rerunning the generation command rather than rewriting evidence.
