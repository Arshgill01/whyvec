# Phase 06: Product hardening and distribution

## Entry conditions

- End-to-end diagnosis, repair, refusal, validation, and benchmark paths exist.
- Core schemas and artifact formats are versioned.

## Deliverables

- Pinned container and native installation paths.
- Supported-platform matrix and diagnostic self-check.
- Artifact retention, export, redaction, and cleanup policies.
- Cache integrity and invalidation.
- Stable CLI help, exit codes, and progress behavior.
- Compatibility tests across supported Clang patch releases and targets.
- Security regression suite.
- Reproduction bundles and judge/test sandbox.
- Complete operational and release logs.

## Edge cases

- Interrupted analysis and process cleanup.
- Read-only filesystems and restricted containers.
- Large IR and optimization records.
- Unicode, spaces, and symlinks in paths.
- Offline operation and absent container runtime.
- Partial cache writes and version upgrades.
- Private source export and redaction.
- Unsupported compiler wrappers or SDK layouts.

## Exit gates

- A clean environment installs and executes documented fixtures.
- Supported platforms produce semantically equivalent reports.
- Security suite passes under the distribution configuration.
- Interrupted runs leave recoverable, correctly classified artifacts.
- Exported bundles contain no configured secrets or absolute private paths.
- Product help and documentation match actual behavior and exit codes.

## Retained implementation evidence

- `containers/judge/Dockerfile` pins the base image digest, Rust 1.96.1,
  Clang/LLVM 21, Codex CLI 0.144.3, Python dependencies, helpers, and plugin.
- `scripts/demo --ci` verifies the clean non-interactive product path; the
  default mode performs the live installed-plugin handoff.
- `scripts/check_portable_evidence.py` rejects home/temp paths, raw reasoning
  events, and token telemetry in shareable Codex and real-world bundles.
- GitHub's compiler-product and judge-container jobs exercise the full product,
  not only Rust unit tests.
