# Threat model

## Assets

- User source code and repository integrity.
- Credentials and environment secrets available to the invoking process.
- Host filesystem, processes, network identity, and compute resources.
- Correctness and provenance of compiler evidence.
- Integrity of caches, reports, logs, and exported reproduction bundles.

## Trust boundaries

Untrusted inputs include:

- source repositories and submodules;
- `compile_commands.json` and response files;
- compiler wrappers and executable paths;
- Clang plugins, pass plugins, sysroots, headers, and generated sources;
- environment variables and working directories;
- optimization records imported from outside the current analysis;
- model-generated patches and commands;
- fixture archives and reproduction bundles.

Trusted computing base:

- WhyVec's argument parser, policy engine, workspace manager, hasher, variant generator, report serializer, and pinned toolchain image;
- the operating-system isolation primitives actually enabled for the run.

Clang/LLVM is trusted for the observed compiler result but not assumed immune to malicious input, crashes, or resource exhaustion.
The same boundary applies to GCC and the TypeScript native compiler. GCC
observation uses a fixed argv in a fresh output directory and accepts no
project-supplied plugin or response-file arguments; operating-system sandboxing
for optimization-only adapters remains distribution-hardening work.

## Threats and controls

### Arbitrary command execution

Threat: compilation entries may invoke shells, package managers, downloaders, wrappers, or arbitrary executables.

Controls:

- parse argv arrays directly;
- accept an allowlisted compiler frontend by digest;
- reject shell flags, unknown wrappers, and plugin-loading options;
- disable network access in isolated execution;
- expose every rejected command component in a redacted policy report.

Current build-causality status: Cargo, direct Clang/GCC, and the logical
TypeScript project command are explicit, tokenized, run with an environment
allowlist, bounded, and forced offline for dependency resolution where
applicable. They execute through mandatory fingerprinted Bubblewrap with all
namespaces unshared, including the network; a read-only host root; a private
`/tmp`; and host writes limited to the fresh detached worktree and build-output
directory. There is no unsandboxed fallback. Seccomp and enforced cgroup quotas
remain residual distribution-hardening work.

### Source-tree mutation

Threat: compilers, plugins, build scripts, or agent commands modify user files.

Controls:

- read-only source mount or verified copy;
- separate working and artifact directories;
- pre/post repository digest or Git status check;
- output-path rewriting;
- explicit source-edit authority only in the later Codex repair stage.

Every build adapter materializes each variant in a fresh detached Git worktree
at the recorded base commit. It never applies an atom to the user's worktree or
index. Bubblewrap exposes the original repository only through the read-only
host-root mount. Worktree removal failure is a fatal analysis result.

### Path and cleanup attacks

Threat: traversal, symlinks, hardlinks, or unresolved variables redirect writes or deletion.

Controls:

- canonicalize allowed roots;
- use operating-system-created unique directories;
- reject paths escaping allowed roots;
- avoid following symlinks during artifact writes;
- validate cleanup targets by identity, not string prefix alone;
- keep interrupted artifacts recoverable until explicit cleanup.

### Secret leakage

Threat: flags, environment, paths, compiler output, or source excerpts expose credentials.

Controls:

- environment allowlist rather than full inheritance;
- secret-pattern and key-name redaction;
- source excerpt opt-out;
- separate local and exportable reports;
- never log authentication headers, tokens, SSH commands, or raw credential files.

### Resource exhaustion

Threat: hostile templates, compiler bugs, or process trees consume unbounded resources.

Controls:

- CPU, wall-clock, memory, file-size, process-count, and output-size limits;
- process-group termination;
- bounded search spaces;
- partial-result finalization;
- per-analysis quotas and cache accounting.

### Evidence forgery and cache poisoning

Threat: stale or attacker-provided artifacts are presented as results of the current inputs.

Controls:

- content-addressed artifacts;
- include every semantic compiler input in the cache key;
- record producer version and schema;
- verify digests before report assembly;
- never trust optimization records not produced inside the active analysis workspace;
- sign exported bundles when signing support is configured.

### Model overreach

Threat: the agent invents a contract, runs an unsafe command, or hides a failed test.

Controls:

- deterministic report as the source of compiler facts;
- explicit agent authority and prohibited actions;
- command policy and user-visible execution;
- append-only validation log;
- required negative and fallback tests;
- typed refusal when repository evidence is incomplete.

### Undefined behavior amplification

Threat: an experimental assumption or repair turns latent undefined behavior into a misleading optimization success.

Controls:

- sanitizers on fixtures and repaired code where supported;
- do not execute semantic variants as user programs unless policy permits;
- distinguish compiler acceptance from source correctness;
- preserve the original fallback;
- refuse source annotations without full caller contracts.

## Residual risks

- Compiler vulnerabilities reachable through crafted source or IR.
- Platform-specific pointer-provenance rules in runtime range checks.
- Build semantics hidden in unsupported wrappers or generated steps.
- Incomplete caller discovery across dynamic linking, callbacks, or FFI.
- Behavior differences outside executed tests.
- Benchmark distortion from system load and hardware policy.

Residual risks must appear in user-facing output when relevant to a decision.
