# Product specification

## Re-founded product boundary

[ADR 0005](decisions/0005-causal-compiler-debugger.md) broadens the product from
one vectorization family into a causal compiler debugger. The two top-level jobs
are:

1. explain which working-tree edit sets are sufficient to produce a selected
   build diagnostic relative to a passing base;
2. explain which typed compiler assumptions are sufficient to change a selected
   optimization decision.

The original specification below remains the contract for the LLVM
alias/trip-count optimization pack. The executable Cargo/rustc build query is
specified in [BUILD_CAUSALITY.md](BUILD_CAUSALITY.md).

## Product thesis

Compiler optimization remarks answer what happened. WhyVec experimentally identifies a tested condition under which the compiler makes a different decision, then helps a repository-aware agent determine whether and how that condition can be enforced safely.

## Primary audience

- C and C++ developers maintaining performance-sensitive native code.
- Scientific-computing and FFI maintainers whose scalar parameters are passed by pointer.
- Performance engineers investigating missed auto-vectorization.
- Coding agents expected to optimize code without inventing alias guarantees.

## Core job

Given a source loop and its real build configuration:

1. Establish whether the selected compiler pipeline vectorizes it.
2. Search a declared family of controlled semantic assumptions.
3. Identify a sufficient assumption set that changes the outcome.
4. Explain exactly what the experiment did and did not establish.
5. Derive a source-level obligation when the access model supports it.
6. Enable Codex to inspect the repository, select or refuse a repair, and validate the result.

## Product surfaces

### Command line

```console
whyvec analyze SOURCE:LINE [--format human|json]
whyvec doctor [--format human|json]
whyvec explain-build --diagnostic CODE [--at PATH] [--format human|json] -- COMMAND...
whyvec replay-build REPORT.json
whyvec explain-opt SOURCE:LINE --function NAME --parameter NAME:INDEX... \
  --transformer PATH --identity-tool PATH [--format human|json]
whyvec replay-opt REPORT.json
whyvec observe-gcc-opt SOURCE:LINE --function NAME [--llvm-report REPORT.json]
whyvec replay-gcc-opt REPORT.json
whyvec derive-obligation OPTIMIZATION_REPORT.json [--format human|json]
whyvec replay-obligation REPORT.json
```

`analyze` is the normal C golden path: it discovers and fingerprints exactly
one safe compilation-database entry, infers the containing function and direct
C pointer-parameter mapping, locates bundled helpers, runs optimization and
obligation analysis, and emits a compact versioned Codex packet. `doctor`
checks the pinned platform, compiler, build, plugin, and helper surface. The
expert `explain-*`, `observe-*`, and `derive-*` commands emit concise human output
or their versioned JSON model. Replay commands verify retained evidence and
semantic reproduction. Progressive `inspect`, `compare`, `verify`, and
`artifacts` convenience commands remain future product-hardening surfaces; they
are not current CLI commands.

### Codex integration

The `whyvec-optimize` skill orchestrates diagnosis, repository inspection, repair selection, patching, and validation. It must use the deterministic CLI for every compiler claim and retain a schema-valid repository action trace linking the exact candidate to its validation evidence.

WhyVec emits one compact agent packet, so the skill does not require users to
manually locate five reports and helpers. `./scripts/demo` exercises the
installed plugin in a fresh Codex session; the sanitized observable record is a
product artifact, while hidden reasoning and token telemetry are not.

### Machine report

The JSON report is the stable integration surface. It includes evidence strength, toolchain provenance, experiment deltas, loop identity, findings, candidate obligations, decline reasons, and required verification.

## Canonical user journey

1. A developer identifies a hot scalar loop.
2. WhyVec reproduces the build and confirms the missed vectorization.
3. WhyVec evaluates supported counterfactuals with one declared delta per experiment.
4. WhyVec finds that modeling a pointer-loaded bound as `noalias` changes the loop to vectorized.
5. Access analysis determines whether the relevant source relationship can be expressed as a bounded non-overlap obligation.
6. Codex inspects callers and contracts.
7. Codex chooses one of:
   - enforce an existing contract with a justified annotation;
   - introduce a runtime guard and retain the original fallback;
   - change an API while updating all callers;
   - refuse because the obligation cannot be established or enforced.
8. WhyVec reruns the original and repaired configurations.
9. Correctness tests cover the optimized and fallback branches.
10. Benchmarks report measured distributions with environment metadata.
11. A noisy benchmark selects retention/refusal instead of upgrading a covered
    correctness result into a performance claim.

The pinned SuperLU SAXPY case demonstrates the equally valid real-world refusal
journey: the real CMake compilation database and repository test pass, the
selected cleanup loop is observed missed, the declared finite counterfactual
search finds no successful assumption, replay matches, and no obligation or
repair is invented.

## Required successful result

A successful diagnosis contains:

- a reproducible baseline miss;
- a matched loop in every compared variant;
- a declared, isolated assumption delta;
- a changed compiler decision;
- the smallest sufficient set found in the declared search;
- a clear evidence-strength label;
- a candidate obligation or a typed reason why one cannot be derived;
- verification requirements for any repository change.

## Required decline behavior

WhyVec must decline rather than guess when:

- no compilation command is unambiguously selectable;
- the compiler or target is unsupported;
- command execution violates policy;
- the source loop cannot be matched to IR with adequate confidence;
- the baseline already vectorizes;
- the baseline fails to compile;
- variants change unrelated compiler inputs;
- multiple source loops collapse into an ambiguous optimized loop identity;
- the selected loop is removed before vectorization analysis;
- inline assembly, volatile, atomics, concurrency, setjmp/longjmp, signals, or undefined behavior invalidate the supported model;
- access extents or loop bounds cannot be derived safely;
- every supported counterfactual still misses;
- a counterfactual crashes or times out;
- the apparent result is non-deterministic across confirmation runs.

Each decline includes a stable code, human explanation, retained evidence, and actionable next investigation when available.

## Differentiation

WhyVec is not:

- a formatter for optimization remarks;
- an LLM prompt around compiler output;
- a general source-to-source vectorizer;
- an automatic `restrict` inserter;
- a replacement for profilers or formal verification;
- a claim that one compiler experiment reveals all legal optimizations.

Its distinct mechanism is controlled counterfactual compilation with explicit evidence semantics and repository-aware enforcement.

## Quality bar

- Identical inputs produce identical semantic reports.
- Human and JSON output never disagree.
- Every speed claim is reproducible from retained commands and measurements.
- A judge or maintainer can run a pinned fixture without reconstructing a toolchain.
- Refusal paths are designed and demonstrated with the same care as success paths.
