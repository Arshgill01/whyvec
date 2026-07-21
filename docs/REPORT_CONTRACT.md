# Report contract

## Re-foundation status

The checked-in `whyvec-report.schema.json` is the version 1 contract for the
LLVM vectorization pack. It cannot serialize build-causality queries, generic
compiler observations, three-valued experiment verdicts, adapter identity, or
pipeline fidelity. [ADR 0005](decisions/0005-causal-compiler-debugger.md)
requires a major-version report schema before the shared experiment runtime is
considered complete. No generic query may be forced into the version 1 loop
fields.

The executable Cargo/rustc, Clang, GCC, and TypeScript build adapters emit the deliberately
separate [build report schema](../schemas/whyvec-build-report.schema.json). Its
`2.0.0-dev` identifier makes the incompatibility explicit while diagnostic,
file/hunk intervention, and artifact contracts are hardened. Nested hunk
refinements retain their parent sufficient file set, fixed atoms, parsed Rust
syntax groups (or explicit text fallback groups), underlying exact hunks,
independent search trace, and full-patch removal witness. It does not silently
redefine the version 1 optimization report.

The executable Clang/LLVM development vertical similarly emits the separate
[optimization report schema](../schemas/whyvec-optimization-report.schema.json)
with `2.0.0-dev`. It retains pipeline fidelity, typed candidates, monolithic
and replay baselines, stable loop identity, three-valued experiment verdicts,
confirmation consistency, tool fingerprints, structured optimization records,
and immutable artifacts. These two development reports must still converge on
the adapter-aware major-version envelope before the shared schema gate closes.

When initial loop identity is absent or ambiguous, `subject` and
`replay_baseline` are null, pipeline fidelity is `not_evaluated`, and the report
contains a stable `baseline.loop_absent` or `identity.ambiguous` decline. This
shape records that selection stopped; it does not synthesize a loop identity or
claim split-pipeline equivalence. If identity becomes unresolved only after a
typed delta, that experiment has null outcome, a typed identity decline, and an
unresolved verdict.

The development build report also retains SHA-256-addressed intervention and
compiler-stream artifacts, adapter-owned diagnostic and tool identities, an
aggregate input and command digest, replay limits, and a normalized semantic digest.
`replay-build` verifies content before executing and rejects input, toolchain,
or semantic drift rather than reporting a reproduced result.

Build toolchain provenance contains a stable adapter name, named driver and
compiler identities, digested support files where applicable, and the Bubblewrap invocation/resolved binary,
digests and version plus asserted network, host-root, and temporary-filesystem
isolation properties. The sandbox fingerprint participates in the command and
semantic digests, so replay refuses a changed or missing containment provider.

The development optimization report now carries the repository and source
locations needed for local replay, bounded-search limits, and a normalized
semantic digest. `replay-opt` verifies all declared artifacts before executing,
then rejects source drift, any Clang/`opt`/transformer/identity-helper
fingerprint drift, or a changed semantic result. The digest intentionally
excludes analysis identifiers, repository and artifact locations, and artifact
references while retaining source and pipeline digests, stable loop identity,
outcomes, search trace, minimality, finding, and decline semantics.

The GCC observation adapter emits
[a dedicated report](../schemas/whyvec-gcc-observation-report.schema.json).
It retains GCC's native compressed optimization record and a decompressed JSON
copy, generator metadata, stable pass names, selected source remarks, tool
fingerprints, semantic replay, and an optional integrity-checked LLVM
comparison. A comparison says `agrees`, `diverges`, or `not_comparable`; it does
not translate GCC records into LLVM interventions or source obligations.

The first source access model emits the separate
[obligation report](../schemas/whyvec-obligation-report.schema.json). Positive
reports contain one `derived_obligation` and no decline; refusal reports contain
one stable `obligation.*` decline and no obligation. Both shapes retain the
upstream optimization identity and semantic digest, exact source digest,
fingerprinted Clang, AST artifacts, semantic digest, and replay input. The
runtime guard section is a checked enforcement plan, not a generated patch or
an assertion about callers.

Guarded source actions emit a separate
[validation report](../schemas/whyvec-validation-report.schema.json). Its
evidence strength is `validated_on_covered_executions`. It links the source
obligation, exact fixture source digests, normalized commands, compiler
identity, differential and sanitizer branch counts, fast/fallback optimization
records, raw benchmark samples, dispersion, decision rule, and environment.
Correctness or compiler-record failures prevent report creation. Benchmark
noise produces `noise_decline`, not a speed claim.

## Compatibility

The report uses semantic versioning in `schema_version`.

- Patch changes clarify descriptions without changing accepted documents.
- Minor changes add optional fields or enum values that tolerant consumers can ignore.
- Major changes alter required fields, meanings, or structural compatibility.

Consumers must reject unsupported major versions and preserve unknown fields when round-tripping reports.

The canonical schema is [schemas/whyvec-report.schema.json](../schemas/whyvec-report.schema.json).

## Required sections

### Identity

- report and analysis identifiers;
- schema and WhyVec versions;
- lifecycle state;
- timestamps and duration;
- source repository and source digest metadata.

### Toolchain provenance

- compiler and optimizer paths, versions, and binary digests;
- target triple and CPU features;
- normalized flags and command digest;
- container image and host metadata;
- relevant environment allowlist.

### Subject

- canonical source location;
- function identity;
- loop structural fingerprint;
- nesting path;
- mapping confidence and supporting signals.

### Baseline

- classification;
- compiler outcome and structured remarks;
- vector factor or miss reason when available;
- artifact references and digests;
- confirmation-run consistency.

### Search space

- assumption family and version;
- candidate assumptions;
- subset strategy;
- evaluated and skipped subsets;
- stop condition;
- exhaustiveness status.

Build-query nested search records syntax-group identifiers separately from
their member Git hunks. Minimality applies to the declared executable groups,
not to arbitrary textual lines inside one parsed item.

### Experiments

Every experiment records its exact delta, verifier result, execution result, matched-loop outcome, artifacts, and confound checks.

### Finding

A finding states:

- evidence strength;
- smallest sufficient set found;
- minimality classification;
- human summary constrained by that classification;
- direct and indirect compiler changes observed;
- confidence and caveats.

### Obligation

The obligation is either:

- `derived`, with source entities, ranges, predicates, preconditions, enforcement options, and unsupported behaviors; or
- `declined`, with a stable reason and missing evidence.

### Verification requirements

The report defines required build, compiler, behavior, overlap, fallback, sanitizer, and benchmark checks. It does not claim they have run until a later verification report records them.

### Decline

Any declined analysis contains one primary stable code, optional contributing codes, a concise explanation, retained evidence, and safe next actions.

## Stable decline families

The canonical meanings and evidence requirements live in [DECLINE_CODES.md](DECLINE_CODES.md).

- `input.*` — invalid or ambiguous source and command inputs.
- `policy.*` — denied execution, path, plugin, wrapper, or environment behavior.
- `toolchain.*` — unsupported, missing, mismatched, or unstable compiler components.
- `baseline.*` — compile failure, already vectorized, no matched loop, or inconsistent outcome.
- `variant.*` — invalid delta, verifier failure, crash, timeout, or confound.
- `identity.*` — ambiguous or low-confidence loop matching.
- `search.*` — no successful assumption, incomplete search, or resource bound.
- `obligation.*` — unsupported access, extent, arithmetic, concurrency, or language semantics.
- `verification.*` — required post-repair evidence missing or failed.

## Human rendering

The terminal renderer follows this order:

1. baseline outcome;
2. counterfactual outcomes;
3. smallest sufficient set found;
4. evidence-strength statement;
5. candidate obligation or decline;
6. required next action;
7. provenance summary and artifact path.

Warnings affecting correctness appear before performance information.

## Redaction

Reports may contain source paths, flags, defines, SDK paths, and excerpts. Export policy supports:

- path relativization;
- source excerpt omission;
- environment allowlisting;
- secret-pattern redaction;
- artifact digest retention when content is omitted.

Redaction must not make two distinct semantic inputs appear identical. The unredacted local digest remains available for comparison.
