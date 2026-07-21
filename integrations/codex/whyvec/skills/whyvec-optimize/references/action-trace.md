# Reading a repository action trace

The bundled planner produces a create-new `whyvec-agent-trace` document. It is a deterministic handoff and audit record, not an autonomous source edit.

## What the planner establishes

- optimization and obligation reports replay with the supplied WhyVec executable;
- exact upstream report paths remain discoverable alongside their identifiers and semantic digests;
- every listed report artifact remains inside its report directory and matches its retained digest and size;
- a validation report, when supplied, links the same obligation and satisfies the guarded fixture gates;
- the exact candidate passed the retained default-ABI compile and execution before a guarded action can be selected;
- tracked text occurrences and obvious indirect, dynamic, and external-linkage uncertainty are inventoried;
- `restrict`, guarded versioning, API change, and refusal receive explicit decisions and reasons;
- a supplied candidate source is retained as a digest and normalized diff.
- every guarded validation command retains its zero exit status plus stdout and stderr digests and sizes.

The validation report is linked by both analysis identifier and whole-report SHA-256 because repair-validation reports do not have a public replay command.

## What Codex must still establish

The planner cannot prove caller completeness, API intent, language semantics, FFI contracts, concurrency safety, portability of integer-address comparisons, or suitability of the candidate patch for the repository. Inspect those facts directly before editing.

`closed_within_tracked_sources` means only that the preliminary scan found no uncertainty pattern. Never convert it directly into a `restrict` contract. External linkage, generated code, callbacks, weak symbols, dynamic loading, macro expansion, and future API callers require separate evidence.

## Decision meanings

- `validated_guarded_runtime`: the supplied guarded candidate has linked differential, fallback, sanitizer, compiler-record, and benchmark evidence. Codex still reviews and applies the repository-appropriate patch.
- `validation_required`: a supported runtime obligation exists, but the guarded candidate has not passed all linked validation gates.
- `refuse`: deterministic obligation derivation declined. Do not synthesize a contract to bypass the refusal.

Keep the trace with the final diff and validation artifacts. If the patch changes, create a new trace rather than editing the retained one. Report replay retains new analyses, so do not invoke the planner during an answer-only review without source-change authority.
