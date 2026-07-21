# AGENTS.md

These rules apply to the entire WhyVec repository. A nested `AGENTS.md` may add stricter component-specific rules but may not weaken the invariants below.

## Mission

Build a counterfactual optimization debugger that produces reproducible compiler evidence and enables Codex to make safe, repository-aware optimization decisions.

Correctness, semantic honesty, and reproducibility outrank apparent success. A refusal backed by evidence is better than a patch based on an unjustified assumption.

## Non-negotiable semantic language

- Say **observed** when describing a baseline compiler result.
- Say **tested sufficient assumption** when a shadow compilation changes the result.
- Say **smallest set found** unless the declared finite search was exhaustively evaluated.
- Say **minimal within the declared search space** only when exhaustive search evidence is retained.
- Say **candidate obligation** until source access analysis derives an enforceable condition.
- Say **validated on covered executions** for tests. Never claim tests establish full semantic equivalence.
- Reserve **proved** for a result backed by a named proof system and a retained proof artifact.

## Architectural boundaries

Keep these concerns separate:

1. **Toolchain capture** fingerprints Clang, LLVM, target, flags, environment, source digest, and compilation database entry.
2. **Baseline analysis** observes the unmodified build.
3. **Variant generation** changes exactly the assumptions declared by an experiment.
4. **Outcome comparison** compares the same loop identity across baseline and variants.
5. **Obligation derivation** translates supported IR-level assumptions into source-level conditions.
6. **Agent orchestration** inspects callers, selects or refuses a repair, modifies source, and validates the repository.
7. **Reporting** serializes facts without upgrading their evidence strength.

The model must never be the source of truth for compiler outcomes, loop identity, compiler flags, or experiment success.

## Source and workspace safety

- Treat analyzed repositories, build commands, response files, compiler plugins, and generated scripts as untrusted input.
- Perform shadow compilation in a fresh, bounded workspace outside the source tree.
- Mount or copy source read-only whenever the execution environment permits.
- Never overwrite source, `compile_commands.json`, build artifacts, or user configuration during diagnosis.
- Reject compilation entries that invoke shells, unknown wrappers, network fetches, or compiler plugins until the policy layer explicitly permits them.
- Resolve and validate all paths before cleanup. Never recursively delete an unresolved, root, home, repository-root, or workspace-root path.
- Redact environment variables and command arguments matching secret patterns before logging.

## Counterfactual integrity

Every experiment must record:

- baseline analysis identifier;
- source and compilation-command digests;
- compiler binary digest and version;
- target triple, CPU features, optimization level, and relevant environment;
- selected loop identity and debug location;
- exact assumption delta;
- exact compiler invocation after normalization;
- optimization record and exit status;
- whether unrelated IR changed before the controlled optimization stage;
- loop-matching confidence and ambiguity;
- result classification and decline reason, if any.

Changing flags, source text, target features, or pass pipelines outside the declared delta invalidates the comparison.

## Obligation integrity

- Do not equate parameter-level LLVM `noalias` with an arbitrary pairwise range condition.
- Derive a range obligation only when all relevant reads, writes, bases, extents, and loop bounds are understood by a supported access model.
- Preserve integer overflow, negative-bound, zero-trip, pointer-provenance, volatile, atomic, concurrency, and exceptional behavior considerations.
- A runtime guard must be evaluated before any transformation that assumes the guarded fact.
- The fallback must retain the original loop and observable behavior.
- Do not add `restrict`, `llvm.assume`, alias metadata, vectorization pragmas, or unsafe intrinsics without a repository-level contract that justifies them.

## Repository workflow

Before changing implementation:

1. Read [PLAN.md](PLAN.md) and the relevant phase document.
2. Read the affected architecture, semantic, report, testing, and threat-model sections.
3. Inspect applicable decision records.
4. Add or identify a fixture that would fail without the change.
5. Record material experiment or design evidence in the relevant log.

Keep commits coherent and attributable to one capability or policy change. Update documentation and schemas in the same commit when behavior or evidence shape changes.

## Validation requirements

Run the narrowest checks that establish the change, followed by repository-level validation when shared contracts change.

At minimum, validate:

- formatting and static checks;
- unit tests for pure domain logic;
- golden report compatibility;
- baseline/variant isolation;
- positive and negative compiler fixtures;
- fallback behavior for overlapping inputs;
- refusal behavior for unsupported access patterns;
- deterministic output under repeated identical runs;
- schema compatibility and log completeness.

Do not report a check as passing unless the command completed successfully.

## Documentation and logs

- Keep logs append-only. Correct a mistaken entry with a new correction entry.
- Use UTC timestamps in ISO 8601 format.
- Include commands, versions, artifact paths, and result summaries.
- Never store secrets, raw tokens, private source, or unredacted environment dumps.
- Link decisions to experiments and validation evidence.
- Do not leave placeholder text, fictional benchmark results, or claims unsupported by retained artifacts.

## Definition of done

A capability is complete when its phase exit gates pass, report and CLI contracts agree, positive and refusal paths are tested, threat-model implications are addressed, documentation is current, and the validation evidence is recorded.
