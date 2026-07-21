# ADR 0005: Generalize WhyVec into a causal compiler debugger

## Status

Accepted.

## Context

The original architecture treats one LLVM loop-vectorization counterfactual as the product. Direct experiments confirm that the mechanism is real: Clang 21 reports an uncountable loop for the canonical bound-alias fixture, while an isolated `noalias` intervention on the bound argument changes the matched loop to vectorized under the recorded optimizer pipeline.

That evidence does not resolve the product limitations. A single C aliasing family has a narrow audience, source enforcement is language-specific, compiler remarks sometimes already expose useful causes, and structured compiler interfaces are becoming ordinary agent infrastructure.

The durable, non-commoditized primitive is executed counterfactual compilation: change one declared input, rerun the same compiler question, and compare the same observation.

## Decision

WhyVec will implement a causal compiler debugger with:

- compiler/build adapters;
- typed observation identities;
- typed intervention providers;
- a shared isolated experiment engine;
- a three-valued oracle;
- finite sufficient-set search with explicit minimality;
- causal evidence graphs;
- language- and family-specific source actions or refusals.

The first query types are build causality and optimization causality. The existing LLVM alias/trip-count work becomes an optimization experiment pack rather than the top-level architecture.

Cross-language support is adapter-specific. The system will not normalize away compiler semantics or claim that one LLVM source obligation applies to every frontend.

## Consequences

- Loop-specific concepts move out of the shared domain core and into the LLVM vectorization pack.
- Fixture and toolchain profiles become per-adapter and per-case.
- Pipeline fidelity becomes mandatory evidence.
- Clang C/C++, Cargo/rustc, and TypeScript may share experiment infrastructure while exposing different observations and interventions.
- The agent layer consumes causal evidence but remains unable to invent compiler facts.
- Existing phase documents remain historical input and must be revised where they conflict with this decision.

## Rejected alternatives

### Keep WhyVec as a C-only vectorization utility

Rejected because the impact ceiling and fixture dependence remain structural.

### Add more languages by parsing their diagnostic text

Rejected because it creates a generic wrapper without stable identity, interventions, or source semantics.

### Build an LSP or MCP bridge

Rejected because language-server access, structured diagnostics, and semantic editing are already being integrated directly into coding agents.

### Make an LLM generate counterfactuals without typed packs

Rejected because compiler claims would become non-reproducible and unsafe.

