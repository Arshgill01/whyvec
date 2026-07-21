# Contributing to WhyVec

WhyVec accepts changes that improve counterfactual accuracy, semantic integrity, agent usefulness, reproducibility, or product clarity.

## Before changing code

1. Read [AGENTS.md](AGENTS.md).
2. Identify the owning phase in [PLAN.md](PLAN.md).
3. Read the relevant semantic and architectural contracts.
4. Add or select a fixture representing the behavior.
5. Decide which report fields, decline reasons, or threat boundaries change.

## Change requirements

- Keep the compiler observation, counterfactual inference, and repository decision separate.
- Add a negative or refusal test alongside a new successful analysis path.
- Preserve raw compiler artifacts used to justify golden outputs.
- Update schemas and compatibility fixtures together.
- Add an ADR for durable changes to architecture, evidence semantics, toolchain policy, or agent authority.
- Record validation commands and results in the append-only validation log.

## Commit structure

Prefer one coherent capability or policy per commit. Use imperative commit subjects. Do not combine formatting sweeps or unrelated refactors with semantic changes.

## Pull requests

Describe:

- the compiler or user problem;
- the exact evidence transition;
- supported and declined cases;
- semantic and security implications;
- validation commands and retained artifacts;
- report-schema or compatibility impact.

Reviewers should reject changes that make the output sound more certain than the retained evidence.
