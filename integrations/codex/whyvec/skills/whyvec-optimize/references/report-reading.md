# Reading a WhyVec report

## Trust order

1. Validate `schema_version` and the report schema.
2. Verify compiler binary digest, version, target, CPU features, flags, pass pipeline, source digest, and normalized compilation entry.
3. Verify baseline and variant use the same source loop identity with adequate confidence.
4. Inspect the declared assumption delta and isolation checks.
5. Read raw optimization-record references before relying on the human summary.
6. Treat a changed outcome as evidence about that exact experiment only.

## Evidence vocabulary

| Report concept | Defensible statement | Invalid leap |
| --- | --- | --- |
| Baseline outcome | Clang observed the loop as missed under the recorded configuration. | The loop can never vectorize. |
| Successful variant | The tested assumption was sufficient to change the recorded outcome. | Real callers satisfy the assumption. |
| Smallest set found | No smaller successful set was found by the recorded search. | This is the only or globally minimal cause. |
| Candidate obligation | The supported access model derived a condition for enforcement. | Parameter `noalias` means only these two byte ranges differ. |
| Covered validation | The retained tests passed for named executions. | The patch is formally equivalent. |

## Mandatory consistency checks

- Every experiment references the same baseline analysis identifier.
- Every result has retained command, exit status, stderr, and optimization-record artifacts.
- Exactly the declared assumption differs in a valid one-delta experiment.
- A vectorized classification includes loop-vectorization evidence, not only a faster benchmark.
- Minimality language matches the search completion status.
- Declines retain a typed code and remediation evidence, not an empty result.

If human and JSON outputs disagree, treat the JSON plus retained raw artifacts as authoritative and report the renderer defect.
