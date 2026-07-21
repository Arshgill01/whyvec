# Repair selection policy

## Contract hierarchy

Prefer evidence in this order:

1. language or type-system invariant;
2. documented public API precondition enforced across boundaries;
3. explicit runtime check dominating the optimized path;
4. repository-wide caller inspection with closed-world justification;
5. tests and current caller behavior as supporting evidence only.

Do not convert levels 4 or 5 into a public `restrict` contract when external, indirect, generated, or future callers are possible.

## Repair matrix

| Condition | Preferred action |
| --- | --- |
| Existing contract already guarantees the required non-aliasing relationship | Express the contract narrowly and validate every caller. |
| Relationship can be checked safely and cheaply at runtime | Add guarded fast path; preserve original fallback. |
| Check cost exceeds or erases the measured gain | Retain original loop and document the decline. |
| Access range cannot be computed without overflow or undefined behavior | Refuse automatic versioning. |
| Volatile, atomic, concurrent, device, or signal-visible accesses are involved | Refuse unless a domain-specific model establishes safety. |
| External callers cannot be audited | Do not add a global `restrict` promise. |
| Compiler result changes but end-to-end performance does not | Retain original implementation. |

## Runtime guard requirements

A guard must compare integer address intervals using a representation and platform policy the repository supports. It must:

- handle zero-length ranges without forming invalid end pointers;
- reject or safely handle byte-count multiplication overflow;
- avoid dereferencing the pointer-loaded bound after entering a path that assumes it immutable unless dominance is established;
- account for writes that can reach the bound object through any alias, not only the named parameter;
- preserve the original loop in the fallback.

The patch description must state whether the check is a portable C rule, a platform-specific address-space assumption, or a project contract.
