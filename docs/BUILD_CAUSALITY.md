# Build causality

## Product question

An ordinary compiler reports diagnostics for the final program it receives. A
large agent-authored change can produce dozens of errors across files, and the
first printed error is not necessarily the authored change that made the build
fail.

WhyVec build causality asks:

> Which independently executable edit set is sufficient to produce this exact
> compiler diagnostic relative to a passing base revision?

The result is counterfactual compiler evidence. It is not a semantic judgment
that the edit is wrong, and it is not a claim that the selected edit is the only
possible cause.

## Command

```console
whyvec explain-build \
  --base HEAD \
  --diagnostic E0308 \
  --at src/lib.rs \
  -- cargo check -p api
```

If a code and path still match more than one diagnostic, WhyVec lists stable
diagnostic identities. Rerun with the identity:

```console
whyvec explain-build \
  --diagnostic rustc:E0308:0123456789abcdefabcd \
  -- cargo check
```

`--format json` emits the same report model shown in the terminal. The retained
report is written beneath `.whyvec/analyses/<analysis-id>/report.json`.

## Executed protocol

1. Resolve the repository and exact base commit.
2. Capture tracked changes from `base → current working tree`, including staged
   and unstaged content.
3. Capture untracked, non-ignored files as explicit atoms.
4. Create a detached temporary worktree at the base commit.
5. Run the Cargo command with no atoms and require a passing baseline.
6. Create a fresh worktree, apply every atom, and require the selected
   diagnostic to exist in the candidate build.
7. Parse Cargo `compiler-message` JSON and establish a stable diagnostic
   identity.
8. Evaluate atom subsets in deterministic cardinality-first order.
9. Classify each run through a three-valued oracle.
10. For every sufficient set found, apply its complement to the base and test
    whether removing the set from the full patch makes the diagnostic disappear.
11. Record other full-patch diagnostics that disappear in that removal witness.
12. Persist the report and remove every temporary worktree.

The user's source tree, index, and branch are never reset, checked out, or
patched.

`.whyvec/` is a reserved analysis-state namespace and is excluded from both
tracked and untracked change atoms. A tracked rename or copy crossing that
boundary is refused because partially ignoring it would corrupt the captured
change.

Tracked patches, untracked file bytes and permissions, and in-repository
symlink targets are snapshotted before the baseline executes. Every subset run
therefore receives the same captured intervention even if the source working
tree changes while the analysis is running.

## Diagnostic identity

Line numbers alone are unstable when edit subsets add or remove surrounding
source. Raw rendered text also contains worktree paths and formatting.

The Cargo/rustc adapter fingerprints:

- compiler adapter;
- rustc error or lint code;
- severity;
- normalized diagnostic message;
- Cargo target name;
- repository-relative primary source path;
- normalized primary-span label;
- normalized source excerpt at the primary span.

It deliberately excludes line and column numbers from identity while retaining
them for display. This allows a diagnostic to move without becoming a different
observation. Two diagnostics with the same code but different source excerpts
remain distinct.

Diagnostic identity is adapter-owned. Clang SARIF, TypeScript compiler API, and
GCC JSON diagnostics will require different identity evidence.

## Change atoms

The current adapter uses one atom per changed tracked path and one atom per
untracked file. Renames and copies remain grouped with both paths. Binary
changes are preserved through Git binary patches.

File atoms are deliberately honest but not the final granularity. They answer
which changed files are sufficient. A subsequent refinement layer must split a
successful text-file atom into dependency-aware hunks or syntax edits while
retaining the file-level result as its parent evidence.

WhyVec does not currently claim that one line inside a sufficient file is the
cause.

## Three-valued build oracle

For the selected diagnostic `D`, a subset build returns:

- `observed` — `D` is present under the same stable identity;
- `not_observed` — the build succeeds and `D` is absent;
- `unresolved` — the build cannot answer whether `D` depends on the subset.

An unresolved result includes:

- another compiler failure prevents a successful counterexample;
- an atom cannot be applied independently;
- the build times out;
- output truncation prevents reliable observation;
- diagnostic identity becomes ambiguous;
- the tool or policy fails.

Unresolved is never converted into `not_observed`. An unresolved smaller subset
prevents a minimality claim.

## Sufficiency and removal witnesses

For a passing base `B`, full edit set `F`, target diagnostic `D`, and candidate
set `S`:

```text
compile(B)      does not emit D
compile(B + F)  emits D
compile(B + S)  emits D
```

This establishes that `S` is sufficient to produce `D` under the recorded
build.

WhyVec additionally evaluates:

```text
compile(B + (F - S))
```

If `D` disappears, the report records a removal witness. Diagnostics present in
the full patch but absent from this build are linked as co-suppressed
observations. This is stronger and more useful than selecting the first printed
compiler error, but it remains experimental evidence rather than formal actual
causality.

## Search integrity

- Candidate IDs are sorted before enumeration.
- Subsets are evaluated by increasing cardinality and lexicographic order.
- Every run uses a detached worktree at the same base commit.
- Duplicate candidate IDs and invalid resource limits are rejected before the
  compiler runs.
- Search budgets and skipped declared subsets remain visible.
- `unique_minimal_in_declared_search` requires every smaller subset and every
  same-cardinality alternative to resolve without another success.

## Process and repository safety

- Commands are tokenized and never passed through a shell.
- Builds receive a small environment allowlist rather than the agent's full
  secret-bearing environment.
- Cargo network resolution is forced offline.
- Non-JSON Cargo message formats are rejected instead of silently losing
  diagnostic evidence.
- stdout and stderr are drained with retained-size bounds.
- timeouts terminate the process group, not only its parent process.
- untracked symlinks resolving outside the repository are rejected.
- unmerged paths and non-UTF-8 paths currently decline.
- every temporary Git worktree is explicitly removed, including after build
  failures.

Operating-system sandboxing of arbitrary `build.rs` processes is not yet
implemented. Cargo offline mode does not prevent a build script from creating
its own network connection. Run the adapter only on repositories whose build
execution is trusted until the sandbox gate is executable.

## Current refusal and limitation surface

- The base revision must pass the exact Cargo command.
- The full change must fail and contain one uniquely selected diagnostic.
- Cargo is the only executable build adapter.
- Patch granularity is currently file-level.
- Dirty submodules, unmerged files, non-UTF-8 paths, and special untracked files
  are unsupported.
- Compiler output beyond the configured bound is not silently treated as
  complete evidence.
- Non-monotonic compiler recovery may produce unresolved subsets and weaker
  minimality.
- A sufficient set does not establish developer intent, desired API design, or
  the correct repair.

## Agent handoff

The deterministic report gives Codex:

- the target diagnostic identity and source span;
- sufficient edit sets and their files;
- unresolved competing subsets;
- the removal witness;
- diagnostics suppressed with the target;
- the original build command and base commit.

Codex then inspects the semantic relationship between those edits, determines
whether the change or its consumers should be repaired, applies the repository
patch, and reruns the original build. GPT-5.6 must not rewrite the causal report
or upgrade sufficiency into correctness.
