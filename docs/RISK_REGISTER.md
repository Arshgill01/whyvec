# Risk register

| ID | Risk | Consequence | Detection | Mitigation | Exit evidence |
| --- | --- | --- | --- | --- | --- |
| R-001 | Baseline loop already vectorizes | Demo and diagnosis are invalid | Optimization record before search | Classify and decline `baseline.already_vectorized`; maintain fixture | Golden decline report |
| R-002 | Parameter `noalias` is translated into a narrower source promise | Unsafe `restrict` or guard | Compare IR assumption semantics with access summary | Label IR assumption separately; derive obligations through a typed family | Obligation unit and refusal tests |
| R-003 | Variant changes more than one semantic input | False causal attribution | Structural pre-optimization IR diff | Reject any delta beyond declared attributes | Isolation test artifacts |
| R-004 | Loop identity drifts across variants | Compare unrelated loops | Multi-signal identity score | Decline below confidence threshold | Ambiguous nested-loop fixture |
| R-005 | Earlier passes cause the changed result | Direct-cause claim is overstated | Pass and IR comparison | Report final sufficiency and intermediate transformation; avoid direct-cause wording | Confound-aware report fixture |
| R-006 | Runtime guard introduces undefined pointer arithmetic | Repair breaks defined behavior | Static review, sanitizers, boundary tests | Target-aware integer address model with checked arithmetic; fallback on uncertainty | Overflow and provenance tests |
| R-007 | Cached bound changes overlap behavior | Silent semantic regression | Bound-inside-output regression | Cache only in dominated fast path; preserve original fallback | Adversarial overlap test |
| R-008 | Caller discovery is incomplete | Annotation contract is unjustified | Linkage and reference analysis reports unknown edges | Prefer guard or refuse; never infer closed world silently | Dynamic/FFI caller refusal fixture |
| R-009 | Compiler command executes hostile code | Host compromise or leakage | Policy inspection before launch | Allowlist frontend, reject wrappers/plugins, sandbox and redact | Threat test suite |
| R-010 | Report schema and terminal wording diverge | Agent and human act on different claims | Render both from one typed model | Golden semantic comparison | Renderer tests |
| R-011 | Benchmark noise becomes a false speed claim | Misleading impact | Distribution and environment checks | Retain raw samples; report dispersion; decline noisy results | Reproducible benchmark log |
| R-012 | Search pruning is described as minimal | Inflated claim | Search trace and stop condition | Use precise minimality enums and include skipped subsets | Search property tests |
| R-013 | Toolchain drift changes outcome | Reproduction failure | Binary digest and container image comparison | Pin complete toolchain and report all inputs | Cross-run reproduction bundle |
| R-014 | Macro, template, or inlining mapping is ambiguous | Wrong source patch | Debug/AST/IR correlation confidence | Decline uncertain mapping and expose candidates | Mapping refusal fixtures |
| R-015 | Model contribution appears decorative | Weak product coherence | Agent workflow trace | Make repository contract discovery, strategy selection, patching, and validation core | Recorded end-to-end Codex session |
| R-016 | Scope remains a synthetic compiler puzzle | Weak potential impact | Real-repository case review | Maintain real FFI/scientific patterns and explain engineering cost | Case study with retained source provenance |
| R-017 | Formal-sounding language exceeds evidence | Trust loss | Documentation and report lint | Enforce evidence vocabulary in types, tests, and review | Claim-language validation |
| R-018 | Exported artifacts reveal private source | Confidentiality breach | Export manifest review | Redaction modes, digests, path relativization, excerpt opt-out | Security export tests |

## Maintenance

Add risks when a new compiler family, assumption, repair strategy, toolchain, or execution authority enters the system. Close a risk only with linked validation evidence; do not delete historical entries.
