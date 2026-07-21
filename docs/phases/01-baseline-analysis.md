# Phase 01: Reproducible baseline analysis

## Entry conditions

- Phase 00 exit gates pass.
- Pinned Clang/LLVM distribution is identified by binary and image digest.
- Execution policy can isolate compiler processes and artifacts.

## Deliverables

- Rust workspace and CLI entry point.
- Source-location and compilation-command selection.
- Shell-free command normalization and response-file policy.
- Toolchain fingerprinting.
- Unique content-addressed artifact workspace.
- Baseline IR and serialized optimization record capture.
- Stable source-loop identity with confidence signals.
- Classifications for missed, vectorized, compile-failed, absent, and ambiguous loops.
- Human and JSON baseline reports rendered from one model.

## Edge cases

- Multiple commands for one source file.
- Relative include paths and working directories.
- C and C++ driver aliases pointing to the same compiler.
- ThinLTO, PCH, modules, unity builds, and generated source.
- Macros and nested loops sharing debug locations.
- Compiler wrappers, plugins, and response files.
- Existing runtime-versioned vectorization.
- Optimization records with multiple miss causes.
- Target feature differences and absent debug columns.

## Exit gates

- Pinned fixtures reproduce identical semantic reports across fresh runs.
- Source tree remains byte-identical after analysis.
- Baseline artifacts contain normalized command, verified IR, optimization record, and digests.
- Ambiguous inputs decline with stable codes and actionable details.
- Already-vectorized loops are never forwarded into counterfactual search.
- Security tests demonstrate shell metacharacters and denied plugins are not executed.
