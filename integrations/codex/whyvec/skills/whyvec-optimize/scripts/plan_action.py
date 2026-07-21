#!/usr/bin/env python3
"""Build a deterministic, evidence-linked repository action trace for Codex."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path


TEXT_SUFFIXES = {
    ".c",
    ".cc",
    ".cpp",
    ".cxx",
    ".h",
    ".hh",
    ".hpp",
    ".hxx",
    ".json",
    ".md",
    ".py",
    ".rs",
    ".rst",
    ".sh",
    ".toml",
    ".txt",
    ".yaml",
    ".yml",
}


def digest(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def load_report(path: Path, versions: set[str]) -> tuple[dict[str, object], str]:
    content = path.read_bytes()
    report = json.loads(content)
    if report.get("schema_version") not in versions:
        raise RuntimeError(
            f"unsupported report version at {path}: {report.get('schema_version')}"
        )
    verify_artifacts(path, report)
    return report, digest(content)


def verify_artifacts(report_path: Path, report: dict[str, object]) -> None:
    artifacts = report.get("artifacts")
    if not isinstance(artifacts, list):
        raise RuntimeError(f"artifact manifest is absent: {report_path}")
    for raw in artifacts:
        if not isinstance(raw, dict) or not isinstance(raw.get("path"), str):
            raise RuntimeError(f"malformed artifact manifest: {report_path}")
        artifact = (report_path.parent / raw["path"]).resolve(strict=True)
        if not artifact.is_relative_to(report_path.parent.resolve(strict=True)):
            raise RuntimeError(f"artifact escapes report directory: {artifact}")
        content = artifact.read_bytes()
        if len(content) != raw.get("size") or digest(content) != raw.get("sha256"):
            raise RuntimeError(f"artifact digest or size mismatch: {artifact}")


def replay_report(whyvec: Path, command: str, path: Path, semantic_digest: object) -> None:
    completed = subprocess.run(
        [str(whyvec), command, str(path)],
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"{command} rejected {path}: {completed.stdout}{completed.stderr}"
        )
    result = json.loads(completed.stdout)
    if result.get("matched") is not True or result.get("semantic_digest") != semantic_digest:
        raise RuntimeError(f"{command} did not reproduce report semantics: {path}")


def tracked_files(repository: Path) -> list[Path]:
    completed = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=repository,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.decode(errors="replace"))
    files = []
    for item in completed.stdout.split(b"\0"):
        if not item:
            continue
        path = (repository / item.decode()).resolve(strict=True)
        if not path.is_relative_to(repository):
            raise RuntimeError(f"tracked path escapes repository: {path}")
        files.append(path)
    return files


def discover_references(repository: Path, function: str) -> dict[str, object]:
    occurrences = []
    uncertain = []
    token = re.compile(rf"\b{re.escape(function)}\b")
    call = re.compile(rf"\b{re.escape(function)}\s*\(")
    files = tracked_files(repository)
    for path in files:
        if path.suffix.lower() not in TEXT_SUFFIXES:
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        for number, line in enumerate(lines, 1):
            if not token.search(line):
                continue
            kind = "call_or_declaration" if call.search(line) else "reference"
            if path.suffix.lower() in {".md", ".rst", ".txt"}:
                kind = "documentation"
            elif "test" in path.parts or path.name.startswith("test"):
                kind = "test"
            occurrences.append(
                {
                    "path": str(path.relative_to(repository)),
                    "line": number,
                    "kind": kind,
                    "excerpt": line.strip()[:240],
                }
            )
            if f"&{function}" in line or "dlsym" in line:
                uncertain.append(
                    {
                        "path": str(path.relative_to(repository)),
                        "line": number,
                        "reason": "indirect or dynamic reference",
                    }
                )
    source_definitions = [
        item
        for item in occurrences
        if item["path"].endswith((".c", ".cc", ".cpp", ".cxx"))
        and item["kind"] == "call_or_declaration"
    ]
    if source_definitions and not any("static" in item["excerpt"] for item in source_definitions):
        uncertain.append(
            {
                "path": source_definitions[0]["path"],
                "line": source_definitions[0]["line"],
                "reason": "external linkage permits callers outside the tracked repository",
            }
        )
    occurrences.sort(key=lambda item: (item["path"], item["line"], item["kind"]))
    uncertain.sort(key=lambda item: (item["path"], item["line"], item["reason"]))
    return {
        "tracked_files_scanned": len(files),
        "occurrences": occurrences,
        "uncertain_edges": uncertain,
        "caller_coverage": "incomplete" if uncertain else "closed_within_tracked_sources",
    }


def validation_ready(
    report: dict[str, object] | None,
    obligation: dict[str, object],
    candidate_digest: str | None,
    *,
    require_measured_improvement: bool = True,
) -> bool:
    if report is None or candidate_digest is None:
        return False
    linked = report.get("obligation")
    commands = report.get("commands")
    command_outcomes = report.get("command_outcomes")
    outcomes = (
        [outcome for outcome in command_outcomes if isinstance(outcome, dict)]
        if isinstance(command_outcomes, list)
        else []
    )
    successful_commands = {
        outcome.get("name") for outcome in outcomes if outcome.get("exit_status") == 0
    }
    required_commands = {
        "abi_compile",
        "abi_execute",
        "differential_compile",
        "differential_execute",
        "production_differential_compile",
        "production_differential_execute",
        "sanitizer_compile",
        "sanitizer_execute",
        "production_sanitizer_compile",
        "production_sanitizer_execute",
        "production_optimization_compile",
        "production_benchmark_compile",
        "production_benchmark_execute",
    }
    command_indices = [outcome.get("command_index") for outcome in outcomes]
    command_ledger_complete = (
        isinstance(commands, list)
        and len(outcomes) == len(command_outcomes) == len(commands)
        and all(isinstance(index, int) for index in command_indices)
        and sorted(command_indices) == list(range(len(commands)))
        and all(outcome.get("exit_status") == 0 for outcome in outcomes)
        and required_commands.issubset(successful_commands)
    )
    differential = report.get("differential", {})
    sanitizer = report.get("sanitizer", {})
    return (
        isinstance(linked, dict)
        and command_ledger_complete
        and report.get("candidate_source_sha256") == candidate_digest
        and linked.get("analysis_id") == obligation.get("analysis_id")
        and linked.get("semantic_digest") == obligation.get("semantic_digest")
        and report.get("evidence_strength") == "validated_on_covered_executions"
        and isinstance(differential, dict)
        and differential.get("fast_paths", 0) > 0
        and differential.get("fallback_paths", 0) > 0
        and differential.get("overflow_refusals", 0) > 0
        and isinstance(sanitizer, dict)
        and sanitizer.get("clean") is True
        and sanitizer.get("covered") == differential
        and report.get("optimization", {}).get("fast_path") == "vectorized"
        and report.get("optimization", {}).get("fallback") == "missed"
        and (
            not require_measured_improvement
            or report.get("benchmark", {}).get("summary", {}).get("classification")
            == "measured_improvement"
        )
    )


def normalized_diff(original: Path, candidate: Path) -> str:
    before = original.read_text(encoding="utf-8").splitlines(keepends=True)
    after = candidate.read_text(encoding="utf-8").splitlines(keepends=True)
    return "".join(
        difflib.unified_diff(before, after, fromfile="a/source", tofile="b/candidate")
    )


def report_path(path: Path, output: Path) -> str:
    resolved = path.resolve(strict=True)
    return os.path.relpath(resolved, output.parent.resolve())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--optimization-report", type=Path, required=True)
    parser.add_argument("--obligation-report", type=Path, required=True)
    parser.add_argument("--validation-report", type=Path)
    parser.add_argument("--whyvec", type=Path, default=Path("whyvec"))
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--candidate-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()

    whyvec = arguments.whyvec
    if whyvec.parent == Path("."):
        resolved = shutil.which(str(whyvec))
        if not resolved:
            raise RuntimeError(f"WhyVec executable is unavailable: {whyvec}")
        whyvec = Path(resolved)
    whyvec = whyvec.resolve(strict=True)
    optimization, _ = load_report(arguments.optimization_report, {"2.0.0-dev"})
    obligation, _ = load_report(arguments.obligation_report, {"2.0.0-dev"})
    loaded_validation = (
        load_report(arguments.validation_report, {"1.1.0"})
        if arguments.validation_report
        else None
    )
    validation, validation_digest = loaded_validation or (None, None)
    replay_report(
        whyvec,
        "replay-opt",
        arguments.optimization_report,
        optimization.get("semantic_digest"),
    )
    replay_report(
        whyvec,
        "replay-obligation",
        arguments.obligation_report,
        obligation.get("semantic_digest"),
    )
    if obligation.get("optimization_analysis_id") != optimization.get("analysis_id"):
        raise RuntimeError("obligation does not link the supplied optimization analysis")
    if obligation.get("optimization_semantic_digest") != optimization.get("semantic_digest"):
        raise RuntimeError("obligation does not link the supplied optimization semantics")
    if obligation.get("decline") is not None or not isinstance(obligation.get("obligation"), dict):
        decline = obligation.get("decline", {})
        action = "refuse"
        action_reason = f"obligation engine declined: {decline.get('code', 'unknown')}"
    else:
        action = "guarded_runtime"
        action_reason = "the derived condition is runtime-enforceable with an unchanged fallback"

    repository = arguments.repository.resolve(strict=True)
    subject = optimization.get("subject")
    if not isinstance(subject, dict):
        raise RuntimeError("optimization report has no selected subject")
    discovery = discover_references(repository, subject["function"])
    candidate = (
        arguments.candidate_source.resolve(strict=True)
        if arguments.candidate_source
        else None
    )
    candidate_digest = digest(candidate.read_bytes()) if candidate else None
    behavior_validated = validation_ready(
        validation,
        obligation,
        candidate_digest,
        require_measured_improvement=False,
    )
    ready = behavior_validated and validation_ready(
        validation, obligation, candidate_digest
    )
    benchmark_declined = (
        behavior_validated
        and validation is not None
        and validation.get("benchmark", {}).get("summary", {}).get("classification")
        == "noise_decline"
    )
    if action == "guarded_runtime" and benchmark_declined:
        action = "refuse"
        action_reason = (
            "benchmark noise or dispersion did not justify guarded repair complexity"
        )
    elif action == "guarded_runtime" and not ready:
        action_reason = "guarded runtime enforcement requires linked behavior and compiler validation"

    alternatives = [
        {
            "strategy": "restrict_annotation",
            "decision": "rejected",
            "reason": (
                "caller coverage is incomplete and LLVM noalias is broader than the derived loop range"
                if discovery["caller_coverage"] == "incomplete"
                else "no repository-supported complete restrict contract was supplied"
            ),
        },
        {
            "strategy": "guarded_runtime",
            "decision": (
                "selected"
                if action == "guarded_runtime" and ready
                else "rejected"
                if action == "refuse"
                else "validation_required"
            ),
            "reason": action_reason,
        },
        {
            "strategy": "api_change",
            "decision": "rejected",
            "reason": "no repository-supported API contract or compatibility authority was supplied",
        },
        {
            "strategy": "refuse",
            "decision": "selected" if action == "refuse" else "not_selected",
            "reason": action_reason if action == "refuse" else "a validated behavior-preserving guard is available",
        },
    ]
    source = Path(optimization["source"])
    finding = optimization.get("finding")
    patch = None
    if candidate:
        source = source.resolve(strict=True)
        if not source.is_relative_to(repository) or not candidate.is_relative_to(repository):
            raise RuntimeError("source and candidate must remain within the repository")
        candidate_path = str(candidate.relative_to(repository))
        patch = {
            "candidate_path": candidate_path,
            "candidate_sha256": candidate_digest,
            "unified_diff": normalized_diff(source, candidate),
        }
    trace = {
        "schema_version": "1.0.0",
        "trace_kind": "codex_repository_action",
        "evidence": {
            "optimization_analysis_id": optimization["analysis_id"],
            "optimization_semantic_digest": optimization["semantic_digest"],
            "optimization_report": report_path(
                arguments.optimization_report, arguments.output
            ),
            "obligation_analysis_id": obligation["analysis_id"],
            "obligation_semantic_digest": obligation["semantic_digest"],
            "obligation_report": report_path(arguments.obligation_report, arguments.output),
            "validation_analysis_id": validation.get("analysis_id") if validation else None,
            "validation_report_sha256": validation_digest,
            "validation_report": (
                report_path(arguments.validation_report, arguments.output)
                if arguments.validation_report
                else None
            ),
            "artifacts_verified": True,
        },
        "compiler_facts": {
            "observed_baseline": optimization["monolithic_baseline"]["classification"],
            "tested_sufficient_assumptions": (
                finding.get("sufficient_assumptions", [])
                if isinstance(finding, dict)
                else []
            ),
            "pipeline_fidelity": optimization["pipeline_fidelity"],
        },
        "candidate_obligation": obligation.get("obligation"),
        "repository_discovery": discovery,
        "alternatives": alternatives,
        "selected_action": (
            "validated_guarded_runtime"
            if action == "guarded_runtime" and ready
            else "refuse"
            if action == "refuse"
            else "validation_required"
        ),
        "patch": patch,
        "validation_commands": validation.get("commands", []) if validation else [],
        "validation_command_outcomes": (
            validation.get("command_outcomes", []) if validation else []
        ),
        "validation_outcomes": {
            "differential": validation.get("differential") if validation else None,
            "sanitizer": validation.get("sanitizer") if validation else None,
            "optimization": validation.get("optimization") if validation else None,
            "benchmark": validation.get("benchmark", {}).get("summary") if validation else None,
        },
        "claim_language": {
            "baseline": "observed",
            "assumption": "tested sufficient assumption",
            "behavior": (
                "validated on covered executions"
                if behavior_validated
                else "not validated"
            ),
        },
        "residual_risks": [
            "unresolved external or dynamic callers do not justify restrict",
            "the integer-address guard is limited to the recorded target policy",
            "covered executions do not establish full semantic equivalence",
        ],
    }
    if arguments.output.exists():
        raise RuntimeError(f"refusing to overwrite action trace: {arguments.output}")
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(trace, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    arguments.output.chmod(0o444)
    print(json.dumps({"output": str(arguments.output), "selected_action": trace["selected_action"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
