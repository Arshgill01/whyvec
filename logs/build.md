# Build log

## 2026-07-21T04:18:59Z — Repository foundation

- Established product, architecture, semantic, experiment, report, agent, testing, and threat contracts.
- Added versioned JSON schemas, pinned-toolchain profile, canonical fixture taxonomy, phase gates, and Codex plugin structure.
- Added a compileable domain crate that enforces evidence ordering, terminal analysis states, and structured decline invariants.

## 2026-07-21T12:54:22Z — Optimization semantic replay

- Extended the development optimization report with local repository context,
  bounded-search replay inputs, and a normalized semantic digest.
- Added `whyvec replay-opt`, which verifies retained artifacts and report
  contents, refuses source or Clang/LLVM/helper drift, re-executes the query,
  and compares compiler/search semantics without treating fresh artifact paths
  as outcome changes.
- Closed the shared immutable runtime's optimization-adapter integration gate;
  retained query-level loop-ambiguity reporting remains an R4 gate.

## 2026-07-21T13:03:39Z — Retained optimization identity declines

- Integrated the LLVM identity helper's typed declines into the optimization
  report instead of converting them to generic tool errors.
- Made the selected subject and replay baseline explicitly absent when no
  unique loop exists, retained raw identity streams, and labeled pipeline
  fidelity `not_evaluated`.
- Represented post-delta identity loss or drift as an unresolved experiment
  with a typed decline and no optimizer outcome.
- Added a manifest-backed two-loop/same-line fixture and semantic replay for its
  retained `identity.ambiguous` result, closing R4's final exit gate.
