# Failure log

## 2026-07-21T04:18:59Z — Conventional input/output alias example rejected

The initial conceptual example used a simple output/input transform. Clang can runtime-version this pattern and vectorize it without source changes, so it cannot demonstrate the target diagnostic gap reliably.

Safeguard: the canonical positive fixture uses a writable array that may alias a pointer-loaded loop bound, and every demo fixture must be confirmed against the pinned compiler profile before acceptance.
