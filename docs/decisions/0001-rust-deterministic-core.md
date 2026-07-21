# ADR 0001: Rust deterministic core

## Status

Accepted.

## Context

WhyVec coordinates untrusted compiler inputs, typed experiment deltas, content-addressed artifacts, subprocess lifecycles, and versioned reports. These paths benefit from explicit types, exhaustive result handling, predictable binaries, and strong testability.

## Decision

Implement the deterministic engine and CLI as a Rust workspace. Keep compiler integrations out of the core domain model and communicate through typed traits and serialized artifacts.

The workspace will separate:

- domain and evidence types;
- command and policy handling;
- Clang/LLVM execution;
- IR variant generation;
- loop identity and result comparison;
- obligation derivation;
- report rendering;
- CLI orchestration.

## Consequences

- Error and decline paths can be modeled exhaustively.
- Distribution can use a single native binary alongside a pinned toolchain image.
- LLVM bindings remain isolated so toolchain coupling does not leak across the product.
- Process and path safety require deliberate implementation despite Rust's memory safety.
