# WhyVec

WhyVec is becoming a causal debugger for compiler decisions.

Compilers describe the final program they received. WhyVec executes controlled
counterfactuals to answer two questions they generally do not answer directly:

> Which tested change is sufficient to produce this compiler failure?

> Which tested compiler assumption is sufficient to change this optimization
> decision under the recorded toolchain, target, flags, and pipeline?

The LLVM alias/trip-count analyzer remains the first deep optimization pack. It
now runs inside an adapter-aware experiment architecture alongside build-causality
queries. Executable fixtures cover Clang 21 and rustc 1.96.1 with LLVM 22; every
result records whether its optimizer pipeline is exact, independently confirmed,
or only a surrogate.

The full re-foundation and defect register live in
[docs/REFOUNDATION_AUDIT.md](docs/REFOUNDATION_AUDIT.md).

## Build causality

The executable build query supports Cargo/rustc, direct Clang and GCC
translation units, and TypeScript projects:

```console
cargo run -p whyvec-cli -- explain-build \
  --base HEAD \
  --diagnostic E0308 \
  --at src/lib.rs \
  -- cargo check
```

WhyVec verifies that the base passes, reconstructs the working-tree change in
isolated detached worktrees, identifies sufficient changed-file sets for the
selected rustc diagnostic, refines successful text files into independently
tested Rust syntax-item groups backed by exact Git hunks, and then removes each
sufficient set from the full patch to measure which diagnostics disappear with
it. See
[Build causality](docs/BUILD_CAUSALITY.md) for the evidence model and refusal
surface.

Every completed query retains content-digested intervention snapshots and raw
compiler streams. Re-execute and compare the normalized semantic result with:

```console
whyvec replay-build .whyvec/analyses/<analysis-id>/report.json
```

TypeScript uses the pinned compiler API adapter:

```console
cd tools/typescript-adapter && npm ci
whyvec explain-build --diagnostic TS2345 --at src/consumer.ts \
  -- whyvec-typescript tsconfig.json
```

GCC and Clang receive native structured-diagnostic flags inside the same
Bubblewrap isolation boundary, for example `-- g++ -fsyntax-only src/main.cpp`.

## Optimization causality

The first executable optimization pack accepts an explicit Clang source/IR
mapping and evaluates typed LLVM parameter assumptions:

```console
whyvec explain-opt src/kernel.c:5 \
  --function add_vectors_ \
  --parameter output:0 --parameter input:1 --parameter count:2 \
  --transformer /path/to/whyvec-llvm-transform \
  --identity-tool /path/to/whyvec-llvm-loop-identity
```

The current development surface requires separately built pinned-LLVM helper
paths. It records `equivalent_confirmed` fidelity and never converts a
successful LLVM assumption into source authorization.

If a source location maps to more than one LLVM loop, the query retains
`identity.ambiguous` with the helper evidence and stops before pipeline replay
or variants. It does not select a nearby loop heuristically.

Completed optimization queries can be re-executed from their retained inputs:

```console
whyvec replay-opt .whyvec/analyses/<analysis-id>/report.json
```

Replay verifies every declared artifact, the report's normalized semantic
digest, the source digest, and all four tool fingerprints before rerunning the
same bounded search. It refuses changed evidence, inputs, tools, or outcomes.

GCC has a separate observation surface because its records and cost model are
not LLVM assumptions:

```console
whyvec observe-gcc-opt src/kernel.c:5 --function add_vectors_ \
  --llvm-report .whyvec/analyses/<llvm-analysis>/report.json
whyvec replay-gcc-opt .whyvec/analyses/<gcc-analysis>/report.json
```

The optional comparison reports only whether the two recorded compiler
classifications agree or diverge for the same canonical source subject.

## Product contract

WhyVec separates three kinds of reasoning that must never be conflated:

1. **Compiler observation** — what the pinned compiler did in the baseline build.
2. **Counterfactual evidence** — what changed when one declared assumption was altered in an otherwise identical shadow compilation.
3. **Repository decision** — whether callers and tests justify `restrict`, require runtime versioning, or make the optimization unsafe to apply.

The compiler engine is deterministic. GPT-5.6 and Codex operate on its evidence; they do not invent the compiler result.

```text
source location
      │
      ▼
baseline compilation ──► missed vectorization record
      │
      ▼
bounded assumption search ──► successful counterfactual
      │
      ▼
semantic obligation ──► repository and caller analysis
      │
      ├── enforceable ──► patch + regression tests + benchmark
      └── unsupported ──► explicit refusal with evidence
```

## Representative interaction

```console
$ whyvec analyze src/kernel.c:12

BASELINE
  status: missed
  compiler: Cannot vectorize uncountable loop

COUNTERFACTUAL SEARCH
  output modeled noalias  -> vectorized, VF=8
  input modeled noalias   -> missed
  count modeled noalias   -> vectorized, VF=8

SMALLEST SUFFICIENT SET FOUND
  { count modeled noalias }

CANDIDATE OBLIGATION
  The object loaded through `count` must not overlap memory modified
  through `output` during the selected loop.

NEXT ACTION
  Repository analysis required. Do not add `restrict` from this result alone.
```

## Engineering principles

- Never mutate the user's source during an experiment.
- Never describe a counterfactual observation as proof that a caller contract is true.
- Never narrow LLVM parameter-level `noalias` into a byte-range promise without separate access analysis.
- Pin and report every compiler input that can affect the result.
- Preserve baseline behavior through an unchanged fallback whenever a runtime guard is selected.
- Treat refusal as a valid, testable product outcome.
- Keep human-readable output and the versioned JSON report semantically identical.
- Preserve every experiment as reproducible evidence rather than transient terminal text.

## Repository map

- [AGENTS.md](AGENTS.md) — operating rules and invariants for every coding agent.
- [PLAN.md](PLAN.md) — living execution plan and acceptance gates.
- [docs/PRODUCT_SPEC.md](docs/PRODUCT_SPEC.md) — audience, product surface, and behavior.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — components, data flow, and boundaries.
- [docs/SEMANTIC_MODEL.md](docs/SEMANTIC_MODEL.md) — evidence strength and alias semantics.
- [docs/EXPERIMENT_PROTOCOL.md](docs/EXPERIMENT_PROTOCOL.md) — reproducible counterfactual procedure.
- [docs/AGENT_CONTRACT.md](docs/AGENT_CONTRACT.md) — Codex/GPT-5.6 responsibilities and refusals.
- [docs/BUILD_CAUSALITY.md](docs/BUILD_CAUSALITY.md) — patch atoms, rustc diagnostic identity, subset search, removal witnesses, and safety boundaries.
- [docs/DECLINE_CODES.md](docs/DECLINE_CODES.md) — stable failure and refusal taxonomy.
- [docs/TEST_STRATEGY.md](docs/TEST_STRATEGY.md) — fixture and adversarial validation matrix.
- [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) — untrusted repository and toolchain risks.
- [docs/phases](docs/phases) — capability phases with entry and exit gates.
- [logs](logs) — append-only build, experiment, validation, failure, and model-use evidence.
- [schemas](schemas) — machine-readable configuration and report contracts.
- [fixtures](fixtures) — positive, fallback, refusal, and already-optimized cases.
- [integrations/codex/whyvec](integrations/codex/whyvec) — installable Codex plugin and workflow skill.
- [crates/whyvec-domain](crates/whyvec-domain) — compileable evidence and lifecycle invariants.
- [crates/whyvec-experiment](crates/whyvec-experiment) — deterministic finite intervention search with a three-valued oracle, evidence-safe minimality, and adapter-neutral immutable artifact storage.
- [crates/whyvec-opt](crates/whyvec-opt) — retained Clang/LLVM optimization-causality query and report assembly.
- [crates/whyvec-build](crates/whyvec-build) — isolated Git/compiler build oracle, adapter-owned diagnostic identity, and causal report generation.
- [crates/whyvec-cli](crates/whyvec-cli) — build and optimization explain/replay command-line product surface.
- [scripts](scripts) — repository and pinned-Clang fixture validation.
- [tools/whyvec-llvm-transform.cpp](tools/whyvec-llvm-transform.cpp) — pinned-LLVM typed IR intervention helper used by the optimization pack.
- [tools/whyvec-llvm-loop-identity.cpp](tools/whyvec-llvm-loop-identity.cpp) — LLVM loop analysis and structural identity helper with ambiguity refusal.

## Current foundation

This repository establishes the executable specification, semantic boundaries, validation surfaces, report schemas, fixture taxonomy, and Codex integration contract from which implementation proceeds. Every implemented behavior must trace back to an acceptance gate in [PLAN.md](PLAN.md) and produce evidence in the appropriate log.

## License

WhyVec is licensed under the [MIT License](LICENSE).
