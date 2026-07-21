# Counterfactual experiment protocol

## Objective

Produce a reproducible comparison in which the declared semantic assumption is the only intended difference between baseline and variant compiler inputs.

## Preconditions

- The repository and source file are readable under analysis policy.
- A compilation database entry is selected unambiguously.
- Clang and LLVM versions satisfy the configured toolchain policy.
- The selected source loop maps to pre-optimization IR with adequate debug information.
- The baseline compiles successfully and the selected loop has a classified outcome.
- Required subprocess isolation and artifact storage are available.

## Baseline procedure

1. Canonicalize the source path without following a symlink outside the allowed repository root.
2. Hash the source, relevant headers where available, compilation database entry, compiler binaries, response files, sysroot, and policy configuration.
3. Tokenize the compilation command without a shell.
4. Reject or explicitly authorize wrappers, plugins, networked build steps, and generated-source dependencies.
5. Redirect outputs into a unique artifact workspace.
6. Preserve language mode, target, CPU features, defines, include order, and optimization-affecting flags.
7. Enable stable debug location emission and serialized optimization records without changing optimization semantics.
8. Produce verified pre-optimization IR.
9. Run the recorded optimization pipeline.
10. Capture exit status, bounded stdout/stderr, remarks, IR, and command metadata.
11. Locate and classify the selected loop.
12. Repeat when confirmation policy requires it and compare semantic outputs.

## Candidate enumeration

For the parameter `noalias` family:

1. Enumerate pointer parameters visible in the selected function.
2. Exclude parameters already marked `noalias`.
3. Exclude parameters whose IR identity cannot be mapped reliably to source.
4. Record parameter index, source name, type, attributes, access summary, and exclusion reasons.
5. Construct the declared subset search according to configuration.

Candidate ordering is stable across identical inputs.

## Variant procedure

For every evaluated assumption set:

1. Copy the baseline pre-optimization IR.
2. Apply typed attribute edits to the selected function arguments.
3. Run the LLVM verifier.
4. Compute a structural diff and confirm no unrelated changes.
5. Run the exact baseline optimization pipeline.
6. Capture the same artifact set as baseline.
7. Match the selected loop using the same identity service.
8. Classify the result.
9. Repeat successful results according to confirmation policy.
10. Finalize the variant artifact directory as immutable.

The production parameter-attribute edit is performed through the matching
LLVM library API, not by rewriting textual IR. Function absence, argument
absence, non-pointer arguments, existing attributes, invalid IR, verifier
failure, and output failure are typed transformation declines.

## Confound detection

Decline or mark inconclusive when:

- compiler path, digest, target, or pass pipeline differs;
- environmental inputs outside the allowlist differ;
- baseline or variant is non-deterministic;
- the loop is inlined, unrolled, distributed, deleted, or merged beyond confident matching before the measured stage;
- the variant changes function attributes beyond the declared set;
- debug information points to multiple candidate source loops;
- the optimization record is absent or malformed;
- an LLVM verifier warning or error occurs;
- an earlier pass changes program structure such that vectorization is not the direct comparable outcome.

An earlier transformation enabled by the assumption is not hidden. The report records the pass sequence when available and says that the assumption was sufficient for the final outcome, not that it acted directly in the vectorizer.

## Result confirmation

A successful result requires:

- the baseline consistently misses;
- the variant consistently vectorizes;
- loop identity meets the configured confidence threshold;
- vectorization evidence includes the optimization record and vector factor when emitted;
- the structural delta before optimization matches the declared assumption set;
- all retained artifacts match their recorded digests.

## Reproduction bundle

An exportable bundle contains:

- normalized baseline and variant commands;
- toolchain fingerprint;
- policy and schema versions;
- source digest and permitted source excerpt;
- pre-optimization IR or its digest when source sensitivity forbids export;
- attribute delta;
- optimization records;
- report JSON;
- replay instructions;
- redaction and omission manifest.

## Experiment log entry

Every completed or interrupted analysis appends an entry to [logs/experiments.md](../logs/experiments.md) with the analysis identifier, toolchain, fixture or repository reference, search summary, artifact location, and conclusion at the correct evidence strength.
