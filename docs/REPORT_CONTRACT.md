# Report contract

## Re-foundation status

The checked-in `whyvec-report.schema.json` is the version 1 contract for the
LLVM vectorization pack. It cannot serialize build-causality queries, generic
compiler observations, three-valued experiment verdicts, adapter identity, or
pipeline fidelity. [ADR 0005](decisions/0005-causal-compiler-debugger.md)
requires a major-version report schema before the shared experiment runtime is
considered complete. No generic query may be forced into the version 1 loop
fields.

The executable Cargo/rustc build vertical currently emits the deliberately
separate [build report schema](../schemas/whyvec-build-report.schema.json). Its
`2.0.0-dev` identifier makes the incompatibility explicit while diagnostic,
atom, and artifact contracts are hardened. It does not silently redefine the
version 1 optimization report.

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
