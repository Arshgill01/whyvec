# Guarded bound-alias repair

This fixture enforces the derived bound-object versus modified-region condition
at runtime under the recorded flat x86-64 `uintptr_t` policy. The fast path
caches the initial bound only after checked multiplication, checked range-end
addition, and disjointness evaluation. The fallback retains the original
pointer-loaded loop.

The harness covers disjoint, bound-before, bound-after, bound-inside-output,
input/output overlap, zero, negative, and address-overflow cases. Differential
agreement is claimed only for those covered executions. The benchmark is
allowed to decline a speed claim when its retained distributions are noisy.
