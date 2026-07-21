# Phase 00: Repository foundation and contracts

> Historical phase: [ADR 0005](../decisions/0005-causal-compiler-debugger.md)
> supersedes the vectorization-only product boundary. Document validation from
> this phase remains useful, but it is not sufficient evidence that the new
> shared compiler experiment foundation is complete.

## Outcome

Establish a repository in which implementation decisions, semantic claims, schemas, tests, security boundaries, agent authority, and evidence logs are explicit and machine-checkable.

## Deliverables

- Root operating rules and execution plan.
- Product, architecture, semantics, experiment, report, agent, test, and threat contracts.
- Accepted architecture decision records.
- Versioned configuration and report schemas.
- Positive, refusal, fallback, and already-vectorized fixture taxonomy.
- Installable and validated Codex plugin/skill scaffold.
- Append-only evidence logs.
- Repository-integrity script and continuous validation workflow.
- Rust workspace architecture decision and component boundaries.

## Adversarial review

- Search for placeholder text and unsupported claims.
- Confirm report terminology cannot conflate observation, sufficiency, obligation, validation, and proof.
- Confirm generated plugin metadata matches the skill.
- Confirm every fixture class maps to a decline or success contract.
- Confirm the threat model covers execution of untrusted compilation inputs.
- Confirm links and JSON documents resolve.

## Exit gates

- Repository-integrity validation passes from a clean checkout.
- Plugin and skill validators pass.
- Every root document links to its canonical detailed contract.
- JSON schemas parse and representative fixture metadata validates structurally.
- No placeholder remains.
- Validation evidence is appended to the logs.
