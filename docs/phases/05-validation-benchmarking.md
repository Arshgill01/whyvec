# Phase 05: Validation and benchmarking

## Entry conditions

- A repository repair exists or is proposed.
- Required verification checks are present in the machine report.
- Baseline and repaired builds are reproducible.

## Deliverables

- Original-versus-repaired differential harness.
- Positive fast-path and adversarial fallback tests.
- Sanitizer configurations for defined fixture inputs.
- Before/after optimization-record comparison.
- Benchmark harness with raw sample retention.
- Environment and CPU-state capture.
- Statistical summary and noise-based decline.
- Verification report linked to the original analysis.

## Edge cases

- Bound inside, before, and immediately after the writable range.
- Input/output overlap independent of bound overlap.
- Negative, zero, tiny, large, and overflow-prone lengths.
- Alignment and allocation variations.
- Warm and cold caches, guard overhead, and scalar tail behavior.
- Different target CPUs and vector factors.
- Sanitizer-induced optimization changes.
- Flaky repository tests and system load instability.

## Exit gates

- Original and repaired implementations agree over the retained differential corpus.
- An overlap case demonstrably executes the unchanged fallback.
- The fast path consistently emits the expected vectorization record.
- Raw benchmark samples and environment are committed or attached as immutable artifacts.
- Reported performance statistics are reproducible and include dispersion.
- Validation failures prevent a success claim and remain in the failure log.
