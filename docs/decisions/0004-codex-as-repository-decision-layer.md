# ADR 0004: Codex as the repository decision layer

## Status

Accepted.

## Context

A compiler experiment cannot establish whether an API contract holds across callers, whether runtime enforcement fits project conventions, or which tests represent required behavior. Hard-coding these repository decisions into the counterfactual engine would either be brittle or falsely authoritative.

## Decision

Expose deterministic findings through versioned JSON and an installable Codex skill. GPT-5.6 inspects the repository, compares repair strategies, explains rejected unsafe alternatives, applies authorized changes, and runs validation. It may not upgrade the evidence strength of the deterministic report.

## Consequences

- The model has a necessary, non-decorative engineering role.
- The integration must support explicit refusal and preserve an auditable workflow trace.
- Agent prompts stay compact because raw compiler artifacts are queryable rather than injected wholesale.
