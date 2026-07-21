# WhyVec

WhyVec is a counterfactual optimization debugger for compiler-guided engineering agents.

Compiler remarks describe the decision a compiler made. WhyVec asks a different question:

> Which tested semantic assumption is sufficient to change that decision under this exact toolchain, target, flag set, and optimization pipeline?

The first fully specified analysis family targets missed Clang loop vectorization caused by a writable array potentially aliasing a pointer-loaded loop bound. WhyVec runs controlled shadow compilations, records the smallest sufficient assumption set found within its declared search space, derives an enforceable source-level obligation where the access pattern supports it, and gives Codex the evidence needed to implement or refuse a repository-level repair.

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
- [docs/DECLINE_CODES.md](docs/DECLINE_CODES.md) — stable failure and refusal taxonomy.
- [docs/TEST_STRATEGY.md](docs/TEST_STRATEGY.md) — fixture and adversarial validation matrix.
- [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) — untrusted repository and toolchain risks.
- [docs/phases](docs/phases) — capability phases with entry and exit gates.
- [logs](logs) — append-only build, experiment, validation, failure, and model-use evidence.
- [schemas](schemas) — machine-readable configuration and report contracts.
- [fixtures](fixtures) — positive, fallback, refusal, and already-optimized cases.
- [integrations/codex/whyvec](integrations/codex/whyvec) — installable Codex plugin and workflow skill.
- [crates/whyvec-domain](crates/whyvec-domain) — compileable evidence and lifecycle invariants.
- [scripts](scripts) — repository and pinned-Clang fixture validation.

## Current foundation

This repository establishes the executable specification, semantic boundaries, validation surfaces, report schemas, fixture taxonomy, and Codex integration contract from which implementation proceeds. Every implemented behavior must trace back to an acceptance gate in [PLAN.md](PLAN.md) and produce evidence in the appropriate log.

## License

WhyVec is licensed under the [MIT License](LICENSE).
