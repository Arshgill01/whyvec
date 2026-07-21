# Architecture

## System context

[ADR 0005](decisions/0005-causal-compiler-debugger.md) generalizes the original
vectorization pipeline below. The shared core now receives a compiler question,
adapter-owned observation identity, and typed intervention provider. The LLVM
loop pipeline remains one optimization pack.

WhyVec sits between an existing native-code repository, a pinned Clang/LLVM toolchain, and a repository-aware coding agent.

```text
                    ┌──────────────────────────┐
                    │ source repository        │
                    │ compile_commands.json    │
                    │ tests + benchmarks       │
                    └────────────┬─────────────┘
                                 │ read-only
                                 ▼
┌──────────────────────────────────────────────────────────────┐
│ WhyVec deterministic engine                                  │
│                                                              │
│ command resolver → baseline → loop identity → variant search │
│        → outcome comparator → obligation derivation          │
│        → report + immutable artifacts                        │
└───────────────────────────────┬──────────────────────────────┘
                                │ versioned JSON
                                ▼
                    ┌──────────────────────────┐
                    │ Codex / GPT-5.6          │
                    │ callers, contracts,      │
                    │ patch, tests, benchmark  │
                    └────────────┬─────────────┘
                                 │ explicit source changes
                                 ▼
                    ┌──────────────────────────┐
                    │ repository validation    │
                    │ build, behavior, remarks │
                    └──────────────────────────┘
```

## Component boundaries

### Build-causality runtime

The Cargo/rustc, direct Clang/GCC, and TypeScript verticals are executable and
share this adapter-neutral runtime:

```text
Git base + working tree
        │
        ▼
file-atom capture ──► detached worktree materializer
        │                         │
        │                         ▼
        │               structured diagnostic oracle
        │                         │
        ▼                         ▼
sufficient-set search ◄── stable diagnostic identity
        │
        ▼
successful file sets ──► zero-context hunk refinement
        │                         │
        │                         ▼
        │                 nested sufficient-set search
        │                         │
        ▼                         ▼
full-patch removal witness ──► co-suppressed diagnostics
        │
        ▼
retained build-causality report
```

Every subset receives a fresh worktree at the same base commit. Build failures
that omit the target diagnostic are unresolved rather than negative evidence.
The implementation and exact safety boundary are specified in
[BUILD_CAUSALITY.md](BUILD_CAUSALITY.md).

Completed build queries retain digested atom payloads and bounded raw compiler
streams. Replay first verifies those files, then requires identical aggregate
input, normalized command, and adapter toolchain fingerprints before re-executing the
search and comparing its normalized semantic digest.

### CLI and application service

Responsibilities:

- parse user intent and source locations;
- load policy and configuration;
- coordinate analysis stages;
- render human output from the report model;
- map internal errors to stable decline codes.

It does not parse compiler prose to infer semantic facts.

The executable `explain-opt` development path currently takes explicit
source-to-IR parameter mappings. It records the observed monolithic baseline,
replays Clang's printable pipeline, validates identity before every optimized
variant, confirms successful outcomes, and retains structured YAML records and
IR. Automatic compilation-database and source-parameter mapping remain product
gates; explicit mappings are evidence inputs, not inferred facts.

Identity selection is a reportable stage outcome. If the helper finds no unique
loop, the report retains its raw identity streams, leaves `subject` and the
split-pipeline baseline absent, records `not_evaluated` pipeline fidelity, and
returns a stable decline without executing any variant. Variant identity loss
or drift is retained as an unresolved experiment, never negative evidence.

`replay-opt` verifies the retained artifact manifest and normalized report
digest, requires the recorded source and Clang/LLVM/helper fingerprints to be
unchanged, reruns the declared bounded search, and compares its semantic
projection. Analysis identifiers, artifact locations and bytes, and repository
location are excluded from that comparison; compiler outcomes, loop identity,
pipeline and source digests, search trace, and conclusions are not.

`observe-gcc-opt` is a sibling observation adapter, not another LLVM
intervention family. It compiles in a fresh temporary output directory with a
fixed `-fsave-optimization-record` command, fingerprints GCC and gzip, maps
GCC's process-local pass identifiers to recorded pass names, and selects
structured remarks by canonical function and source line. Optional comparison
verifies the complete retained LLVM report before labeling the two observed
classifications as agreeing, diverging, or not comparable. `replay-gcc-opt`
refuses artifact, source, toolchain, comparison-report, or semantic drift.

### Compilation command resolver

Responsibilities:

- discover compilation databases;
- enumerate commands matching a source file;
- tokenize arguments without shell execution;
- expand response files under policy;
- distinguish compiler frontends from wrappers;
- normalize paths and environment;
- identify target, language mode, defines, includes, optimization level, and output options;
- remove only output-path flags replaced by the isolated analysis workspace.

Ambiguous commands require explicit selection.

### Toolchain fingerprint service

Fingerprint inputs include:

- compiler path, realpath, file digest, version text, and resource directory;
- target triple, CPU, feature string, sysroot, SDK, and linker-independent frontend flags;
- LLVM `opt` path and version when a split pipeline is used;
- container image digest and host architecture;
- relevant environment allowlist;
- normalized compilation command digest.

Version text alone is insufficient because locally patched compilers may share a version.

### Artifact workspace

Creates a unique analysis directory containing:

```text
analysis.json
baseline/
  command.json
  input.digest
  preopt.ll
  optimized.ll
  remarks.opt.yaml
  stderr.txt
variants/<variant-id>/...
report.json
events.jsonl
```

Artifacts are immutable after finalization. Large or sensitive artifacts may be omitted from export, but their digests and omission reasons remain.

The shared experiment crate owns create-new retention, safe relative artifact
paths, SHA-256 and byte-length references, integrity verification, and
read-only finalization. Adapters decide which compiler inputs and outputs must
be retained; they cannot replace the shared three-valued or content-integrity
contracts.

It also owns argv-only subprocess execution, a clear-by-default environment
allowlist, concurrent bounded stdout/stderr draining, wall-clock timeouts, and
process-group termination. Adapter policy still decides which executable and
arguments are authorized before using this runner.

### Baseline analyzer

Responsibilities:

- reproduce the selected compile under the analysis policy;
- emit debuginfo adequate for loop mapping;
- save pre-optimization IR and serialized optimization records;
- classify compile failure, existing vectorization, missed vectorization, or ambiguity;
- perform confirmation runs when determinism policy requires them.

### Loop identity service

A loop identity combines:

- canonical source path and source digest;
- function linkage/name and debug scope;
- source line/column span;
- loop metadata where available;
- normalized structural fingerprint of the pre-optimization IR;
- parent loop nesting path;
- compiler optimization-record correlation.

Matching returns a confidence level and supporting signals. Low-confidence matches decline comparison.

The LLVM pack's first executable identity boundary resolves an exact IR
function, constructs dominator and loop analyses over pre-optimization IR,
requires exactly one loop whose start debug location matches the selected
source line, and fingerprints the loop's ordered blocks, instruction opcodes,
operand counts, result types, and debug locations. Typed function-argument
attributes do not alter this fingerprint. Zero matches decline
`identity.loop_absent`; multiple matches decline `identity.loop_ambiguous`.

### Variant generator

The generator copies baseline pre-optimization IR and applies a typed assumption delta. The current family adds parameter-level `noalias` to one selected pointer parameter without changing source text, flags, target, or pass pipeline.

Production mutation uses the pinned LLVM C++ API: it parses the module, resolves
the exact function and argument index, requires a pointer argument without an
existing `noalias`, adds the typed attribute, runs LLVM's verifier, and writes
bitcode. The earlier textual transformer remains fixture-only and is not an
authorized production path.

Each variant records:

- source value and IR argument identity;
- attribute before and after;
- patch digest;
- structural diff outside the declared edit;
- verifier result before optimization.

### Search engine

Search spaces are explicit finite sets. The engine:

1. evaluates singleton assumptions in stable order;
2. records every result;
3. optionally expands combinations according to configuration;
4. stops only under a declared strategy;
5. confirms successful candidates;
6. reports `smallest_set_found` or `minimal_in_declared_search` accurately.

No heuristic pruning may be described as exhaustive search.

The shared implementation uses a three-valued oracle. `unresolved` variants
remain distinct from `not_observed`, and any unresolved smaller subset prevents
a minimality claim. Build-causality patch atoms and LLVM assumption sets use the
same cardinality-first deterministic engine.

### Outcome comparator

The comparator uses optimization records and matched IR structure. It distinguishes:

- vectorized at a known fixed or scalable vector factor;
- missed for the same or a different reason;
- loop removed by another transformation;
- loop transformed beyond confident matching;
- compile or optimizer failure;
- non-deterministic outcome.

Assembly inspection may corroborate but does not replace optimization-record evidence.

### Obligation derivation

The supported bound-alias model identifies:

- pointer parameter loaded to obtain a loop bound;
- induction variable and comparison semantics;
- memory writes in the selected loop;
- write base, element size, affine index, and extent;
- reads or writes through the bound-derived pointer;
- integer domains and overflow conditions.

It produces either a typed candidate obligation or a typed decline. It never silently generalizes beyond analyzed accesses.

### Report model

One typed model drives:

- JSON serialization;
- human terminal rendering;
- Codex tool results;
- golden fixtures;
- validation summaries.

Schema versioning follows [REPORT_CONTRACT.md](REPORT_CONTRACT.md).

### Codex integration

The plugin invokes the CLI, reads the JSON report, gathers repository evidence, selects a repair strategy, and validates it. Its authority and refusal rules are defined in [AGENT_CONTRACT.md](AGENT_CONTRACT.md).

## Failure containment

- All subprocess output is size-bounded and separately captured.
- Process trees are terminated as a group on policy violation.
- The source tree is never the compiler output directory.
- Analysis identifiers are unpredictable and collision-resistant.
- Partial analyses finalize with an interrupted state and retained diagnostics.
- Cache entries include every semantic input in their key.
- A cache hit never upgrades evidence gathered under a different toolchain fingerprint.

## Extension architecture

New counterfactual families implement the same lifecycle:

```text
enumerate candidates
→ apply typed delta
→ verify isolation
→ run identical pipeline
→ compare matched outcome
→ derive family-specific obligation or decline
```

Candidate families include alignment, invariant loads, trip-count facts, floating-point semantics, inlining, and runtime-check thresholds. Each requires its own semantic and enforcement model rather than a generic prompt.
