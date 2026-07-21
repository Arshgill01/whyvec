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

## 2026-07-21T13:09:16Z — Mandatory Cargo OS containment

- Routed every Cargo counterfactual through fingerprinted Bubblewrap with all
  namespaces unshared, including network isolation.
- Mounted the host root read-only, exposed only the fresh detached worktree and
  shared Cargo target directory as host-writable, and provided a private
  in-memory `/tmp`.
- Added the sandbox binary identity and asserted isolation properties to build
  toolchain provenance and replay command digests. Missing Bubblewrap has no
  unsandboxed fallback.

## 2026-07-21T13:23:50Z — Rust syntax-aware build interventions

- Added parsed Rust item spans for functions, methods, traits, implementations,
  modules, data types, constants, statics, aliases, and macros.
- Grouped separated zero-context hunks by their smallest enclosing item while
  retaining every exact hunk as the executable payload; malformed Rust and
  other text use explicit one-hunk fallback groups.
- Changed nested causal search and minimality to operate on declared syntax
  groups, with group/hunk membership and removal witnesses serialized in the
  build report and reproduced semantically.
- Closed the remaining R5 grouping gate after the mandatory sandbox milestone.

## 2026-07-21T13:30:07Z — First-class C++ optimization adapter coverage

- Added language identity to every compiler fixture rather than inferring C,
  C++, or Rust solely from the frontend profile.
- Added positive C++ C-linkage and explicit-template-instance fixtures with
  typed LLVM parameter intervention and independent monolithic witnesses.
- Added a C++ macro-origin ambiguity fixture and public retained decline.
- Corrected optimization source artifact naming/media type for C++ inputs.
