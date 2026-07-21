# Test strategy

## Test objectives

Testing must establish that WhyVec reproduces compiler behavior, isolates experimental deltas, reports evidence honestly, derives obligations only for supported semantics, and enables repairs that preserve covered baseline behavior.

## Test layers

### Domain unit tests

- Source-location parsing across relative, absolute, symlinked, and non-UTF-8 paths.
- Compilation argument tokenization without shell expansion.
- Toolchain and command fingerprint stability.
- Assumption-set ordering and minimality classifications.
- Evidence-lattice serialization and rendering.
- Decline-code mapping.
- Address-range arithmetic with zero, negative, maximal, and overflowed extents.
- Schema migration and unknown-field tolerance.

### Compiler fixture tests

Every supported analysis family includes:

- baseline miss with a successful singleton counterfactual;
- multiple successful singletons with deterministic ranking;
- successful pair but no successful singleton;
- no successful supported assumption;
- baseline already vectorized through LLVM runtime checks;
- baseline compile failure;
- loop removed before vectorization;
- ambiguous nested loops on the same source line;
- macro-expanded loop with incomplete debug mapping;
- inline function containing the measured loop;
- distinct compile commands for one source file;
- target-dependent vectorization factor;
- non-deterministic compiler plugin or wrapper, which policy declines.

The LLVM identity helper is additionally tested with two distinct loops that
share one function and debug line. It must return the match count and a typed
ambiguity decline rather than choosing either loop.

### Obligation derivation tests

Supported positive shapes cover:

- signed and unsigned pointer-loaded bounds;
- zero-based and non-zero lower bounds;
- constant positive induction steps;
- one and multiple indexed stores;
- reads from separate arrays that LLVM can runtime-check independently;
- parameter reordered or renamed at source and IR levels.

Required declines cover:

- negative or data-dependent induction steps;
- non-affine indexing;
- pointer-chasing writes;
- writes through function calls without summaries;
- volatile or atomic bound loads;
- volatile or atomic writes;
- signal-visible or concurrently mutated bounds;
- integer overflow in trip-count or byte-extent calculation;
- unknown object size or address space;
- custom allocators with unsupported pointer provenance;
- setjmp/longjmp, callbacks, exceptions, or reentrancy inside the loop;
- inline assembly touching memory;
- undefined baseline behavior detected by sanitizers.

### Repair behavior tests

For a guarded repair:

- non-overlap selects the optimized path;
- overlap with the bound object selects the original fallback;
- overlap between input and output follows original semantics;
- zero and negative bounds preserve behavior;
- smallest and largest supported lengths preserve behavior;
- range arithmetic overflow selects fallback or declines safely;
- null pointers behave consistently with the original preconditions;
- alignment variations do not bypass the guard;
- repeated calls do not retain stale state;
- sanitizer builds remain clean on defined cases.

### Agent workflow tests

Codex must demonstrate:

- selecting a justified annotation when all callers establish the complete contract;
- rejecting that annotation when one caller remains uncertain;
- selecting guarded versioning when enforcement is representable;
- refusing volatile, concurrent, or unbounded cases;
- updating declarations and callers for an API-level repair;
- adding an overlap regression that fails an unsafe cached-bound rewrite;
- rerunning WhyVec and repository checks after the patch;
- distinguishing compiler evidence from repository inference in its explanation.

### Security tests

- response-file path traversal;
- symlink escape from analysis workspace;
- command arguments containing shell metacharacters;
- compiler wrapper that launches a network process;
- malicious Clang plugin flags;
- oversized stdout, stderr, optimization records, and IR;
- fork bombs and child-process escape;
- environment secret redaction;
- artifact-name collisions and Unicode confusables;
- cleanup interruption and partial artifact finalization;
- cache poisoning through omitted semantic inputs.

## Determinism protocol

Run identical analyses in fresh artifact directories and compare normalized reports. Timestamps, durations, random identifiers, absolute ephemeral paths, and run-artifact references are excluded from semantic comparison; toolchain, outcome, search, finding, and obligation semantics must match, while every retained artifact must independently pass its recorded digest and size check.

Both public replay commands first verify retained artifact SHA-256 and byte
lengths, refuse deliberately modified evidence, reject input or toolchain
drift, and only report a match when a fresh execution has the same normalized
semantic digest. Artifact bytes are integrity evidence but are excluded from
the cross-run semantic projection because compiler output may embed fresh
workspace paths.

## Differential behavior protocol

Differential tests execute original and repaired implementations over:

- generated disjoint buffers;
- deliberately overlapping buffers;
- bound pointers placed before, inside, and after writable ranges;
- boundary lengths and values;
- randomized element contents;
- alias layouts selected to exercise every branch.

Crashes, timeouts, sanitizer failures, and output differences are retained as first-class failures. Differential agreement is reported only over executed cases.

## Benchmark protocol

- Benchmark the real build configuration and target.
- Confirm the optimized branch executes for benchmark inputs.
- Separate setup and allocation from the measured kernel.
- Warm the process and collect repeated samples.
- Retain raw samples, compiler fingerprint, CPU model, governor or power mode, affinity, input size, and command.
- Report median and dispersion; avoid a single best run.
- Compare code size and guard overhead alongside throughput.
- Decline a speed claim when noise or environment instability dominates the effect.

## Golden report policy

Golden reports are semantic contracts, not snapshots of incidental paths or wording. Normalize unstable fields. Schema changes require compatibility tests and an explicit decision record when meanings change.
