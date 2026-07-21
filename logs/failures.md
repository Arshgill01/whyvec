# Failure log

## 2026-07-21T04:18:59Z — Conventional input/output alias example rejected

The initial conceptual example used a simple output/input transform. Clang can runtime-version this pattern and vectorize it without source changes, so it cannot demonstrate the target diagnostic gap reliably.

Safeguard: the canonical positive fixture uses a writable array that may alias a pointer-loaded loop bound, and every demo fixture must be confirmed against the pinned compiler profile before acceptance.

## 2026-07-21T08:29:16Z — rustup proxy broken by executable realpath

The first cross-frontend verifier resolved the `rustc` symlink before execution. The resolved binary is the rustup proxy, whose dispatch behavior depends on the invocation name. Executing the realpath therefore printed the rustup version instead of invoking rustc.

Safeguard: compiler adapters preserve the invocation path and separately fingerprint the resolved binary and delegated compiler. Proxy-aware identity is now a named risk and production requirement.
