# Failure log

## 2026-07-21T04:18:59Z — Conventional input/output alias example rejected

The initial conceptual example used a simple output/input transform. Clang can runtime-version this pattern and vectorize it without source changes, so it cannot demonstrate the target diagnostic gap reliably.

Safeguard: the canonical positive fixture uses a writable array that may alias a pointer-loaded loop bound, and every demo fixture must be confirmed against the pinned compiler profile before acceptance.

## 2026-07-21T08:29:16Z — rustup proxy broken by executable realpath

The first cross-frontend verifier resolved the `rustc` symlink before execution. The resolved binary is the rustup proxy, whose dispatch behavior depends on the invocation name. Executing the realpath therefore printed the rustup version instead of invoking rustc.

Safeguard: compiler adapters preserve the invocation path and separately fingerprint the resolved binary and delegated compiler. Proxy-aware identity is now a named risk and production requirement.

## 2026-07-21T09:46:42Z — Timeout killed parent but left descendant holding output pipes

The first bounded-process implementation killed only the direct shell process in its timeout test. The descendant `sleep` process retained the output pipes, so reader threads did not complete until the original five-second command duration elapsed.

Safeguard: Unix subprocesses now start in a dedicated process group and timeout termination targets the entire group. The same test completes at the configured 50-millisecond bound.

## 2026-07-21T09:54:52Z — Retained report re-entered the next causal search

The first repeatability check ran the same query twice in a repository without a `.gitignore`. The second run captured the first run's `.whyvec/analyses/.../report.json` as a new untracked edit atom, changing the declared search space even though the source change was identical.

Safeguard: `.whyvec/` is now an adapter-reserved analysis-state namespace excluded from tracked and untracked atoms. A tracked rename or copy crossing that boundary declines instead of partially capturing the transition. The public validation repeats the query by stable diagnostic identity and compares its causal projection.

## 2026-07-21T09:54:52Z — Untracked atom originally retained a mutable source path

The first Git atom implementation retained the path of an untracked file and copied its content during every subset evaluation. A concurrent working-tree edit could therefore give different subsets different bytes under the same atom identity.

Safeguard: untracked regular-file bytes and permissions, plus validated in-repository symlink targets, are snapshotted before the baseline. A unit test mutates the source after capture and verifies that materialization still uses the captured bytes.

## 2026-07-21T15:45:57Z — Validation temporary storage exhausted

The first R8 repository-wide validation using the default `/tmp` failed with
`Disk quota exceeded`. Pointing `TMPDIR` inside the repository then changed the
Bubblewrap-visible source topology, so the baseline correctly declined rather
than comparing a non-equivalent build.

Safeguard: the complete validation was rerun with `TMPDIR` set to the external
bounded directory `/home/arshdeepsingh/work/whyvec-validation-tmp`. All checks
then passed without changing the repository path seen by isolated builds.
