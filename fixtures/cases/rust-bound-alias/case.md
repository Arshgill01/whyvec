# Rust raw-pointer bound-alias counterfactual

## Purpose

This fixture tests whether the shared LLVM intervention mechanism survives a different frontend without pretending that Rust has C source semantics.

The function uses raw pointers at an FFI-shaped boundary. A safe Rust slice or reference fixture would carry different alias guarantees and must not be treated as equivalent.

## Expected observations

- The real rustc optimized baseline reports an uncountable loop.
- The split rustc-LLVM baseline also remains scalar under the paired LLVM 22 optimizer.
- Adding parameter-level `noalias` to the `count` IR argument changes the split-pipeline outcome to vectorized.
- Adding it only to `input` does not stabilize the trip count.
- Adding it to `output` may also change the outcome, but represents a broader intervention.

## Evidence limitation

The current split pipeline uses the matching LLVM major and a recorded `default<O3>` surrogate. It is evidence that the frontend-neutral IR experiment is viable, not yet evidence that the exact rustc production pipeline would make the same counterfactual decision. The fixture remains labeled `surrogate` until exact replay or independent pipeline equivalence is established.

## Source-action limitation

The result does not authorize converting raw pointers to references, slices, or a different FFI signature. A Rust obligation mapper must inspect ABI exposure, unsafe contracts, callers, and the exact semantic strength of any proposed source type.
