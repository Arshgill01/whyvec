# Guarded bound-alias repair

This fixture enforces the derived bound-object versus modified-region condition
at runtime under the recorded flat x86-64 `uintptr_t` policy. The fast path
caches the initial bound only after checked multiplication, checked range-end
addition, and disjointness evaluation. The fallback retains the original
pointer-loaded loop.

The harness covers disjoint, bound-before, bound-after, bound-inside-output,
input/output overlap, zero, negative, smallest and largest fixture-supported
bounds, and address-overflow cases. Differential agreement is claimed only for
those covered executions. The benchmark is allowed to decline a speed claim
when its retained distributions are noisy.

`candidate.c` is the repository-action form. Its default build preserves the
public `void add_vectors_` symbol and keeps the guard helper internal. A
supplemental branch-witness build overrides only `WHYVEC_GUARD_SCOPE` so the
harness can evaluate the exact guard; the public function and return type do
not change. Production-mode differential, sanitizer, optimization-record, and
benchmark gates compile without that override. `abi_harness.c` separately
compiles and executes the default public signature.
