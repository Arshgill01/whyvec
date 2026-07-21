# ADR 0003: Read-only source and shadow IR variants

## Status

Accepted.

## Context

Editing source to inject `restrict` or pragmas can disturb parsing, macros, formatting, debug locations, and unrelated compiler behavior. Experiments must isolate their semantic delta and never risk user source.

## Decision

Produce verified pre-optimization LLVM IR from the real compilation command, copy it into an isolated artifact workspace, and apply typed attribute deltas to shadow IR. The source repository remains read-only during diagnosis.

## Consequences

- The experimental delta is precisely inspectable.
- Source-level semantics must be derived separately rather than inferred from the IR edit.
- Loop identity must bridge source, baseline IR, variant IR, and optimization records.
- Source modification occurs only through the later authorized agent repair workflow.
