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

## 2026-07-21T15:51:15Z — GitHub runner lacked the mandatory build sandbox

The `Repository integrity` workflow for commit `95bc9e6` passed checkout,
repository validation, formatting, and Clippy, then failed two
`whyvec-build` tests. Both failures were typed artifact-integrity refusals:
the hosted Ubuntu runner could not resolve the required `bwrap` invocation.
The product correctly did not fall back to unsandboxed execution.

Safeguard: the workflow now installs Bubblewrap explicitly and records
`bwrap --version` before running repository checks. The focused fifteen-test
build-adapter suite passed locally with Bubblewrap 0.11.1; completion remains
in verification until the changed workflow passes on GitHub.

## 2026-07-21T15:56:32Z — Installed Bubblewrap was blocked by the runner image

The first workflow repair installed Bubblewrap successfully on
`ubuntu-latest`, which resolved tool discovery. Run `29845981549` still failed
the same two tests with `BaselineFailed([])`: the current `ubuntu-24.04` hosted
image prevented Bubblewrap's unprivileged namespace setup, so the compiler
never ran and no structured diagnostics existed.

Safeguard: the distribution test job now names `ubuntu-22.04` instead of the
moving `ubuntu-latest` label and performs the complete `--unshare-all` sandbox
smoke invocation before repository validation. This retains mandatory
containment and makes an unavailable namespace capability fail at setup rather
than masquerading as a compiler baseline result.

## 2026-07-21T16:12:41Z — Completion audit found incomplete action gates

The R8 planner treated a validation report as action-ready when the required
production commands, fallback witness, sanitizer flag, and compiler records
were present. It did not require every one of the thirteen command outcomes,
a witnessed fast branch, sanitizer coverage equal to the differential corpus,
an overflow refusal, or a benchmark classification of `measured_improvement`.
Consequently, a schema-valid `noise_decline` could still select guarded runtime
versioning, contrary to the agent contract.

Safeguard: guarded selection now requires the complete indexed command ledger,
zero status for all thirteen commands, both branch witnesses, matching
differential/sanitizer coverage, overflow refusal, fast/fallback compiler
records, and measured improvement. A completed `noise_decline` selects refusal
while preserving `validated on covered executions`; missing branch evidence
returns `validation_required` and `not validated`.

## 2026-07-21T16:12:41Z — Checked-in R8 replay evidence was under-verified

Repository validation rehashed the guarded validation artifacts and checked
that `replay.json` said both upstream reports matched. It did not independently
rehash the optimization, obligation, or replay-analysis artifact manifests in
the checked-in R8 bundle. A later missing or modified compiler artifact could
therefore survive the lightweight repository check even though public replay
would refuse it.

Safeguard: repository validation now resolves every artifact beneath its report
directory, rejects escapes and symlinks, checks size and SHA-256, verifies
trace-linked analysis identities and semantic digests, and validates the two
retained replay-analysis reports. A copied-bundle adversarial check removed
`baseline/preopt.ll`; validation refused the bundle with the exact missing
artifact path.

## 2026-07-21T18:21:50Z — Structured outcome parser met pre-identity ambiguity

The first YAML-driven outcome implementation treated simultaneous passed and
missed records at one function/source line as a malformed result. The
ambiguous-loop fixture intentionally has two structural loops at that location,
so record parsing encountered the ambiguity before the identity helper could
retain its typed decline.

Safeguard: structured parsing now reports an ambiguous aggregate outcome while
the structural identity stage remains authoritative and returns
`identity.ambiguous`. Exact duplicate selected records remain rejected, and
malformed, missing, unrelated-loop, and field-name variation tests remain
separate.

## 2026-07-21T19:07:21Z — Demo validation initially benchmarked a stale baseline object

The first `scripts/demo --ci` run copied the retained candidate with
`shutil.copy2`. That preserved an older source modification time than the
already-built baseline object, so Ninja correctly considered the object current.
Differential and direct compiler-record checks used the new source, but the
benchmark executable linked the stale baseline object and returned
`noise_decline` at a median ratio near 1.0. The validation target failed instead
of misreporting improvement.

Safeguard: CI candidate materialization now uses `copyfile`, giving the source a
new modification time and forcing the repository-native rebuild. The repeated
demo observed the candidate object, passed 3,271 covered executions and eleven
mutations, and classified the fresh benchmark as measured improvement. A noisy
future run remains an intentional refusal.

## 2026-07-21T19:15:54Z — Generic validation-plan schema rejected the legacy corpus

The first complete optimization-causality rerun failed when its eleven-case
guarded fixture emitted schema 1.2. The new schema had incorrectly imposed a
twelve-execution minimum even though schema 1.2 delegates the required checks
to a versioned repository validation plan. That would have made the portable
contract depend on WhyVec's demo corpus instead of the reported plan.

Safeguard: version-specific coverage remains exact for schema 1.0 and 1.1,
while schema 1.2 requires witnessed fast and fallback paths and at least two
overflow refusals without dictating a repository's corpus size. The old
eleven-case workflow, the 3,271-case demo, the retained live report, and the
full optimization-causality workflow all pass the corrected schema.

The expanded non-unit-step unit test also exposed that `i += 2` was collected
as though it were an array write. The obligation analyzer now excludes a direct
induction-variable update from memory-write collection; the positive test and
all typed-refusal tests pass.

## 2026-07-21T19:19:23Z — Judge entrypoint lacked its executable mode

GitHub run `29860939811` reached the new judge-container job but exited 126 on
the build step because `containers/judge/build.sh` had been added with mode
`100644`. Docker did not run, so this failure says nothing about the image.

Safeguard: the script is now tracked as `100755`. A replacement push must build
and execute the image before the clean-environment gate is considered closed.
