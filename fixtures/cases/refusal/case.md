# Volatile-bound refusal

The bound is loaded from a volatile object on every loop condition evaluation. Its changing value is part of observable program behavior. Ordinary noalias counterfactual evidence cannot authorize caching the bound or replacing its repeated volatile loads.

The obligation engine must decline with `obligation.volatile_bound`, and the Codex workflow must not propose a guarded cached-bound fast path.
