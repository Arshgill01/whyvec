# Phase 04: Codex and GPT-5.6 repository workflow

## Entry conditions

- The machine report schema is stable for findings and obligations.
- The WhyVec CLI can be invoked deterministically by the plugin.
- Repair and refusal policies are documented.

## Deliverables

- Valid Codex plugin and `whyvec-optimize` skill.
- Report ingestion and compatibility checks.
- Repository caller, declaration, documentation, and test discovery workflow.
- Repair-strategy comparison template.
- Explicit rejection reasons for unsafe alternatives.
- Controlled source-edit and validation sequence.
- Before/after evidence capture.
- Refusal output for incomplete caller or semantic evidence.

## Edge cases

- Indirect, dynamic, weak, or external callers.
- Public headers and ABI compatibility.
- FFI contracts with incomplete source visibility.
- Generated, vendored, or macro-generated functions.
- Existing annotations whose documentation disagrees with callers.
- Repository tests that encode only non-overlap behavior.
- Multiple candidate repairs with different maintenance costs.
- User constraints forbidding API changes or runtime branches.

## Exit gates

- Forward tests show the skill uses deterministic reports for every compiler claim.
- The agent rejects `restrict` when one caller remains uncertain.
- The agent preserves and tests the original overlap behavior in guarded repairs.
- The agent refuses cases the obligation engine declined.
- Every patch trace records inspected callers, strategy comparison, commands, and outcomes.
- Plugin and skill validators pass after installation from a clean checkout.

## R8 current verification evidence

The deterministic handoff is covered by `scripts/verify_optimization_causality.py`.
It produces and schema-validates a guarded selection, a candidate-digest
mismatch that returns `validation_required`, and a volatile-obligation refusal.
The retained positive bundle is under
`evidence/codex-action/2026-07-21/`; it includes replayed upstream reports, the
exact ABI-preserving candidate, external-caller uncertainty, all four strategy
decisions, thirteen compile/execute outcomes, both guarded branches, production
and instrumented sanitizer/compiler coverage, benchmark distributions, and
residual risks.

This does not by itself establish the actual-model exit gate. A fresh
installed-skill GPT-5.6 Codex session must create the candidate, execute its
versioned validation plan, and retain the observable session ledger.
