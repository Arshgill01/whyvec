# Installed-skill Codex run — 2026-07-21

This bundle records an actual fresh `codex exec` run against a clean copy of
`demo/`. It is not a simulated transcript. The plugin was installed through
`codex plugin marketplace add` and `codex plugin add` before the run.

## Product identity

- Codex CLI: `0.144.3`
- Requested model: `gpt-5.6-sol`
- `codex doctor --json` resolved model: `gpt-5.6-sol`
- Codex thread: `019f85ee-5913-7761-b003-30898ee62809`
- Plugin: `whyvec@whyvec-local`, version
  `0.1.0+codex.20260721181611`
- Invocation: `codex exec -m gpt-5.6-sol --dangerously-bypass-approvals-and-sandbox
  --json --output-schema codex-ledger.schema.json -o codex-final-ledger.json -`

The bypass flags were limited to the disposable validation repository. The
product repository was outside that working directory.

## Retained records

- `prompt.txt`: exact user prompt with the disposable path redacted.
- `session.jsonl`: observable completed agent messages, commands, outputs, and
  file changes. Reasoning events, token telemetry, secrets, and machine-private
  paths are deliberately excluded.
- `ledger.json`: final schema-constrained evidence ledger.
- `patch.diff`: complete final repository patch.
- `action-trace.json`: deterministic WhyVec planner decision linked to the exact
  candidate digest and validation report.
- `validation-report.json`: final 1.2 validation report.

The exact model-generated `src/kernel.c` is also retained at
`demo/codex-generated/kernel.c` with SHA-256
`8e6490d23439e1f2b362bc24799443a9b4549557b7da94bdf4f538429431d5cf`.

## Observed result

The model inspected the public header, implementation, FFI wrapper, and overlap
test. It rejected global `restrict` and unconditional bound caching, authored a
checked guarded fast path, and retained the original pointer-loaded loop as the
fallback. The exact candidate was validated on covered executions across 1,127
deterministic differential cases and the same ASan/UBSan corpus. Structured LLVM
records observed the fast loop vectorized at VF=8/IC=4 and the fallback missed.
The retained benchmark classified a measured improvement for the recorded
machine; this is not a universal performance claim.
