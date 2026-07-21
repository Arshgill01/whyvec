# Domain-model agent rules

These rules apply to `whyvec-domain` in addition to the repository-wide instructions.

- Keep this crate independent of Clang, LLVM, subprocess, filesystem, CLI, and model integrations.
- Represent evidence strength, lifecycle states, outcomes, and declines explicitly; do not collapse them into booleans.
- Make invalid state transitions impossible or return a typed error.
- Do not add serialization attributes until the report crate owns a versioned compatibility test.
- Every semantic distinction added here must agree with `docs/SEMANTIC_MODEL.md` and `schemas/whyvec-report.schema.json`.
- Add exhaustive unit tests for transition rules and ordering changes.
