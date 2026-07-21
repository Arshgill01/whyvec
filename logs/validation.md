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
