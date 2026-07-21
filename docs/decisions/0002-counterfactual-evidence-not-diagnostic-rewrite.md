# ADR 0002: Counterfactual evidence, not diagnostic rewriting

## Status

Accepted.

## Context

LLVM already emits human and structured optimization remarks. Reformatting or asking a model to explain those remarks does not reveal semantic facts the compiler omitted.

## Decision

WhyVec's core unit of value is a controlled compiler experiment with an isolated semantic delta and matched before/after outcome. Existing remarks are evidence inputs, not the product.

Every supported finding must trace to retained baseline and variant artifacts. Natural-language explanations are rendered from typed findings.

## Consequences

- A report can remain useful without an LLM.
- GPT-5.6 is responsible for repository reasoning and repair, not compiler-result invention.
- Unsupported counterfactual families decline instead of falling back to prose speculation.
