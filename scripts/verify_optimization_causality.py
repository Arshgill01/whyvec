#!/usr/bin/env python3
"""Exercise the retained public optimization-causality query."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def tool(name: str) -> str:
    resolved = shutil.which(name)
    if not resolved:
        raise RuntimeError(f"required tool is unavailable: {name}")
    return resolved


def run(command: list[str], cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command, cwd=cwd, check=False, capture_output=True, text=True, timeout=180
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def llvm_flags(*libraries: str) -> list[str]:
    return run(
        [
            tool("llvm-config-21"),
            "--cxxflags",
            "--ldflags",
            "--system-libs",
            "--libs",
            *libraries,
        ]
    ).stdout.split()


def compile_helper(source: str, output: Path, *libraries: str) -> None:
    run(
        [
            tool("clang++-21"),
            "-std=c++17",
            str(ROOT / source),
            *llvm_flags(*libraries),
            "-o",
            str(output),
        ]
    )


def invoke(
    binary: Path,
    repository: Path,
    source: str,
    line: int,
    function: str,
    parameters: list[str],
    transformer: Path,
    identity: Path,
) -> dict[str, object]:
    command = [
        str(binary),
        "explain-opt",
        f"{repository / source}:{line}",
        "--repository",
        str(repository),
        "--function",
        function,
    ]
    for parameter in parameters:
        command.extend(["--parameter", parameter])
    command.extend(
        [
            "--transformer",
            str(transformer),
            "--identity-tool",
            str(identity),
            "--format",
            "json",
        ]
    )
    return json.loads(run(command).stdout)


def validate_artifacts(report: dict[str, object]) -> None:
    report_path = Path(str(report["artifact_path"]))
    if not report_path.is_file():
        raise RuntimeError("optimization report was not retained")
    for raw in report["artifacts"]:
        artifact = raw
        path = report_path.parent / artifact["path"]
        content = path.read_bytes()
        if hashlib.sha256(content).hexdigest() != artifact["sha256"]:
            raise RuntimeError(f"artifact digest mismatch: {path}")
        if len(content) != artifact["size"]:
            raise RuntimeError(f"artifact size mismatch: {path}")
        if path.stat().st_mode & stat.S_IWUSR:
            raise RuntimeError(f"finalized artifact remains writable: {path}")


def tamper_copy(report: dict[str, object], destination: Path) -> Path:
    report_path = Path(str(report["artifact_path"]))
    shutil.copytree(report_path.parent, destination)
    return destination / "report.json"


def main() -> int:
    run(["cargo", "build", "--quiet", "-p", "whyvec-cli"])
    binary = ROOT / "target/debug/whyvec"
    with tempfile.TemporaryDirectory(prefix="whyvec-opt-cli-") as temporary:
        temporary_path = Path(temporary)
        retained_bundle = os.environ.get("WHYVEC_RETAIN_AGENT_BUNDLE_ROOT")
        bundle_root = Path(retained_bundle).resolve() if retained_bundle else None
        if bundle_root:
            bundle_root.mkdir(parents=True, exist_ok=False)
        repository = bundle_root / "repository" if bundle_root else temporary_path / "repository"
        repository.mkdir()
        shutil.copytree(ROOT / "fixtures/cases/bound-alias", repository / "bound-alias")
        shutil.copytree(
            ROOT / "fixtures/cases/already-vectorized", repository / "already-vectorized"
        )
        shutil.copytree(ROOT / "fixtures/cases/ambiguous-loop", repository / "ambiguous-loop")
        shutil.copytree(ROOT / "fixtures/cases/cpp-bound-alias", repository / "cpp-bound-alias")
        shutil.copytree(
            ROOT / "fixtures/cases/cpp-template-bound", repository / "cpp-template-bound"
        )
        shutil.copytree(
            ROOT / "fixtures/cases/cpp-macro-ambiguous",
            repository / "cpp-macro-ambiguous",
        )
        shutil.copytree(ROOT / "fixtures/cases/refusal", repository / "refusal")
        shutil.copytree(
            ROOT / "fixtures/cases/bound-alias-repair",
            repository / "bound-alias-repair",
        )
        run(["git", "init", "--quiet"], repository)
        run(["git", "add", "."], repository)

        tool_root = bundle_root / "tools" if bundle_root else temporary_path
        tool_root.mkdir(parents=True, exist_ok=True)
        transformer = tool_root / "whyvec-llvm-transform"
        identity = tool_root / "whyvec-llvm-loop-identity"
        compile_helper(
            "tools/whyvec-llvm-transform.cpp",
            transformer,
            "core",
            "irreader",
            "bitwriter",
            "support",
        )
        compile_helper(
            "tools/whyvec-llvm-loop-identity.cpp",
            identity,
            "core",
            "irreader",
            "analysis",
            "support",
        )

        report = invoke(
            binary,
            repository,
            "bound-alias/kernel.c",
            5,
            "add_vectors_",
            ["output:0", "input:1", "count:2"],
            transformer,
            identity,
        )
        if report.get("pipeline_fidelity") != "equivalent_confirmed":
            raise RuntimeError("pipeline fidelity was upgraded or omitted")
        finding = report.get("finding")
        if not isinstance(finding, dict) or finding.get("sufficient_assumptions") != [
            "parameter.count.noalias"
        ]:
            raise RuntimeError(f"unexpected sufficient assumption: {finding}")
        if report.get("minimality") != "smallest_set_found":
            raise RuntimeError(f"incomplete search overstated minimality: {report.get('minimality')}")
        outcomes = {
            experiment["assumptions"][0]: experiment["outcome"]["classification"]
            for experiment in report["experiments"]
        }
        if outcomes != {
            "parameter.count.noalias": "vectorized",
            "parameter.input.noalias": "missed",
            "parameter.output.noalias": "vectorized",
        }:
            raise RuntimeError(f"unexpected singleton outcomes: {outcomes}")
        for experiment in report["experiments"]:
            outcome = experiment["outcome"]
            if outcome["classification"] == "vectorized" and (
                outcome["confirmation_runs"] != 2 or outcome["consistent"] is not True
            ):
                raise RuntimeError(f"successful variant was not confirmed: {experiment}")
        validate_artifacts(report)
        replay = json.loads(
            run([str(binary), "replay-opt", str(report["artifact_path"])]).stdout
        )
        if replay.get("matched") is not True:
            raise RuntimeError(f"optimization replay did not match: {replay}")
        if replay.get("semantic_digest") != report.get("semantic_digest"):
            raise RuntimeError("optimization replay returned a different semantic digest")

        gcc_report = json.loads(
            run(
                [
                    str(binary),
                    "observe-gcc-opt",
                    f"{repository / 'bound-alias/kernel.c'}:5",
                    "--repository",
                    str(repository),
                    "--function",
                    "add_vectors_",
                    "--gcc",
                    tool("gcc"),
                    "--llvm-report",
                    str(report["artifact_path"]),
                    "--format",
                    "json",
                ]
            ).stdout
        )
        if gcc_report.get("outcome", {}).get("classification") != "missed":
            raise RuntimeError(f"unexpected GCC observation: {gcc_report}")
        if gcc_report.get("comparison", {}).get("relation") != "agrees":
            raise RuntimeError(f"unexpected GCC/LLVM comparison: {gcc_report}")
        validate_artifacts(gcc_report)
        gcc_replay = json.loads(
            run(
                [
                    str(binary),
                    "replay-gcc-opt",
                    str(gcc_report["artifact_path"]),
                ]
            ).stdout
        )
        if gcc_replay.get("matched") is not True:
            raise RuntimeError(f"GCC observation replay did not match: {gcc_replay}")

        cpp_report = invoke(
            binary,
            repository,
            "cpp-bound-alias/kernel.cpp",
            4,
            "add_vectors_cpp",
            ["output:0", "input:1", "count:2"],
            transformer,
            identity,
        )
        if cpp_report.get("finding", {}).get("sufficient_assumptions") != [
            "parameter.count.noalias"
        ]:
            raise RuntimeError(f"unexpected C++ sufficient assumption: {cpp_report}")
        if not any(
            artifact.get("media_type") == "text/x-c++"
            for artifact in cpp_report.get("artifacts", [])
        ):
            raise RuntimeError("C++ source artifact was mislabeled")
        validate_artifacts(cpp_report)

        template_report = invoke(
            binary,
            repository,
            "cpp-template-bound/kernel.cpp",
            3,
            "_Z12template_addIiEvPT_PKS0_PKi",
            ["output:0", "input:1", "count:2"],
            transformer,
            identity,
        )
        if template_report.get("finding", {}).get("sufficient_assumptions") != [
            "parameter.count.noalias"
        ]:
            raise RuntimeError(f"C++ template instance did not reproduce: {template_report}")
        validate_artifacts(template_report)

        declined = invoke(
            binary,
            repository,
            "already-vectorized/kernel.c",
            4,
            "transform",
            ["output:0", "input:1"],
            transformer,
            identity,
        )
        if declined.get("decline", {}).get("code") != "baseline.already_vectorized":
            raise RuntimeError(f"already-vectorized baseline was not declined: {declined}")
        if declined.get("experiments") != [] or declined.get("finding") is not None:
            raise RuntimeError("declined baseline executed or reported counterfactuals")
        validate_artifacts(declined)

        ambiguous = invoke(
            binary,
            repository,
            "ambiguous-loop/kernel.c",
            2,
            "ambiguous",
            ["output:0", "input:1"],
            transformer,
            identity,
        )
        if ambiguous.get("decline", {}).get("code") != "identity.ambiguous":
            raise RuntimeError(f"ambiguous loop was not declined: {ambiguous}")
        if ambiguous.get("pipeline_fidelity") != "not_evaluated":
            raise RuntimeError("ambiguous query overstated pipeline fidelity")
        if ambiguous.get("subject") is not None:
            raise RuntimeError("ambiguous query fabricated a selected loop identity")
        if ambiguous.get("replay_baseline") is not None or ambiguous.get("experiments") != []:
            raise RuntimeError("ambiguous query continued after identity refusal")
        validate_artifacts(ambiguous)
        ambiguous_replay = json.loads(
            run([str(binary), "replay-opt", str(ambiguous["artifact_path"])]).stdout
        )
        if ambiguous_replay.get("semantic_digest") != ambiguous.get("semantic_digest"):
            raise RuntimeError("ambiguous decline did not replay semantically")

        macro_ambiguous = invoke(
            binary,
            repository,
            "cpp-macro-ambiguous/kernel.cpp",
            6,
            "macro_loops",
            ["output:0", "input:1", "count:2"],
            transformer,
            identity,
        )
        if macro_ambiguous.get("decline", {}).get("code") != "identity.ambiguous":
            raise RuntimeError(f"macro-origin loops were not declined: {macro_ambiguous}")
        if macro_ambiguous.get("subject") is not None:
            raise RuntimeError("macro ambiguity fabricated a loop subject")
        validate_artifacts(macro_ambiguous)

        no_success = invoke(
            binary,
            repository,
            "refusal/volatile_bound.c",
            3,
            "update_until",
            ["output:0", "input:1", "bound:2"],
            transformer,
            identity,
        )
        if no_success.get("decline", {}).get("code") != "search.no_successful_assumption":
            raise RuntimeError(f"no-success search did not return a typed conclusion: {no_success}")
        if no_success.get("finding") is not None:
            raise RuntimeError("no-success search fabricated a finding")
        validate_artifacts(no_success)

        obligation = json.loads(
            run(
                [
                    str(binary),
                    "derive-obligation",
                    str(report["artifact_path"]),
                    "--format",
                    "json",
                ]
            ).stdout
        )
        derived = obligation.get("obligation")
        if not isinstance(derived, dict) or derived.get("family") != (
            "bound_object_disjoint_from_modified_region"
        ):
            raise RuntimeError(f"positive obligation was not derived: {obligation}")
        access = derived.get("access_summary", {})
        if access.get("bound_object", {}).get("name") != "count":
            raise RuntimeError(f"bound source entity was not retained: {access}")
        if [write.get("base_parameter") for write in access.get("writes", [])] != [
            "output"
        ]:
            raise RuntimeError(f"write source entities were not retained: {access}")
        if derived.get("candidate_assumption") != "parameter.count.noalias":
            raise RuntimeError("source obligation lost the distinct LLVM assumption")
        validate_artifacts(obligation)
        obligation_replay = json.loads(
            run(
                [
                    str(binary),
                    "replay-obligation",
                    str(obligation["artifact_path"]),
                ]
            ).stdout
        )
        if obligation_replay.get("matched") is not True:
            raise RuntimeError(f"obligation replay did not match: {obligation_replay}")

        retained_validation = os.environ.get("WHYVEC_RETAIN_VALIDATION_ROOT")
        validation_root = (
            Path(retained_validation)
            if retained_validation
            else bundle_root / "validation"
            if bundle_root
            else temporary_path / "guarded-validation"
        )
        validation_meta = json.loads(
            run(
                [
                    "python3",
                    str(ROOT / "scripts/verify_guarded_repair.py"),
                    "--obligation-report",
                    str(obligation["artifact_path"]),
                    "--artifact-root",
                    str(validation_root),
                ]
            ).stdout
        )
        validation_report = json.loads(
            Path(validation_meta["report"]).read_text(encoding="utf-8")
        )
        if validation_report.get("evidence_strength") != (
            "validated_on_covered_executions"
        ):
            raise RuntimeError("guarded validation overstated or omitted evidence strength")
        if validation_report.get("differential") != {
            "executions": 11,
            "fast_paths": 7,
            "fallback_paths": 4,
            "overflow_refusals": 2,
        }:
            raise RuntimeError(f"guarded branch coverage changed: {validation_report}")
        if validation_report.get("optimization") != {
            "fast_path": "vectorized",
            "fallback": "missed",
            "fast_path_line": 42,
            "fallback_line": 47,
        }:
            raise RuntimeError("guarded compiler evidence changed")
        validate_artifacts(validation_report)

        retained_trace = os.environ.get("WHYVEC_RETAIN_AGENT_TRACE_ROOT")
        trace_root = (
            Path(retained_trace)
            if retained_trace
            else bundle_root / "action"
            if bundle_root
            else temporary_path / "agent-trace"
        )
        trace_path = trace_root / "trace.json"
        trace_meta = json.loads(
            run(
                [
                    "python3",
                    str(
                        ROOT
                        / "integrations/codex/whyvec/skills/whyvec-optimize/scripts/plan_action.py"
                    ),
                    "--optimization-report",
                    str(report["artifact_path"]),
                    "--obligation-report",
                    str(obligation["artifact_path"]),
                    "--validation-report",
                    str(validation_meta["report"]),
                    "--whyvec",
                    str(binary),
                    "--repository",
                    str(repository),
                    "--candidate-source",
                    str(repository / "bound-alias-repair/candidate.c"),
                    "--output",
                    str(trace_path),
                ]
            ).stdout
        )
        if trace_meta.get("selected_action") != "validated_guarded_runtime":
            raise RuntimeError(f"guarded action was not selected: {trace_meta}")
        trace = json.loads(trace_path.read_text(encoding="utf-8"))
        if trace.get("repository_discovery", {}).get("caller_coverage") != "incomplete":
            raise RuntimeError("external linkage was upgraded to closed caller coverage")
        restrict = next(
            alternative
            for alternative in trace["alternatives"]
            if alternative["strategy"] == "restrict_annotation"
        )
        if restrict.get("decision") != "rejected" or "incomplete" not in restrict.get(
            "reason", ""
        ):
            raise RuntimeError(f"uncertain callers did not reject restrict: {restrict}")
        if trace.get("patch", {}).get("unified_diff") in {None, ""}:
            raise RuntimeError("guarded action trace did not retain the candidate patch")
        candidate_diff = trace["patch"]["unified_diff"]
        if (
            "+void add_vectors_(" not in candidate_diff
            or "+#define WHYVEC_GUARD_SCOPE static" not in candidate_diff
            or "__attribute__((noinline))" in candidate_diff
        ):
            raise RuntimeError("selected candidate did not retain the public C ABI")

        mismatched_path = temporary_path / "mismatched-agent-trace/trace.json"
        mismatched_meta = json.loads(
            run(
                [
                    "python3",
                    str(
                        ROOT
                        / "integrations/codex/whyvec/skills/whyvec-optimize/scripts/plan_action.py"
                    ),
                    "--optimization-report",
                    str(report["artifact_path"]),
                    "--obligation-report",
                    str(obligation["artifact_path"]),
                    "--validation-report",
                    str(validation_meta["report"]),
                    "--whyvec",
                    str(binary),
                    "--repository",
                    str(repository),
                    "--candidate-source",
                    str(repository / "bound-alias-repair/original.c"),
                    "--output",
                    str(mismatched_path),
                ]
            ).stdout
        )
        if mismatched_meta.get("selected_action") != "validation_required":
            raise RuntimeError("validation for a different candidate authorized a patch")
        mismatched_trace = json.loads(mismatched_path.read_text(encoding="utf-8"))

        volatile_obligation = json.loads(
            run(
                [
                    str(binary),
                    "derive-obligation",
                    str(no_success["artifact_path"]),
                    "--format",
                    "json",
                ]
            ).stdout
        )
        if volatile_obligation.get("decline", {}).get("code") != (
            "obligation.volatile_bound"
        ):
            raise RuntimeError(
                f"volatile access did not produce a typed refusal: {volatile_obligation}"
            )
        if volatile_obligation.get("obligation") is not None:
            raise RuntimeError("volatile refusal fabricated a source obligation")
        validate_artifacts(volatile_obligation)

        refusal_path = temporary_path / "refusal-agent-trace/trace.json"
        refusal_meta = json.loads(
            run(
                [
                    "python3",
                    str(
                        ROOT
                        / "integrations/codex/whyvec/skills/whyvec-optimize/scripts/plan_action.py"
                    ),
                    "--optimization-report",
                    str(no_success["artifact_path"]),
                    "--obligation-report",
                    str(volatile_obligation["artifact_path"]),
                    "--whyvec",
                    str(binary),
                    "--repository",
                    str(repository),
                    "--output",
                    str(refusal_path),
                ]
            ).stdout
        )
        if refusal_meta.get("selected_action") != "refuse":
            raise RuntimeError(f"declined obligation was not refused: {refusal_meta}")
        refusal_trace = json.loads(refusal_path.read_text(encoding="utf-8"))
        if refusal_trace.get("candidate_obligation") is not None:
            raise RuntimeError("refusal trace fabricated a candidate obligation")
        if refusal_trace.get("claim_language", {}).get("behavior") != "not validated":
            raise RuntimeError("refusal trace upgraded behavior evidence")

        schema = json.loads(
            (ROOT / "schemas/whyvec-optimization-report.schema.json").read_text(
                encoding="utf-8"
            )
        )
        try:
            import jsonschema
        except ImportError as error:
            raise RuntimeError("jsonschema is required") from error
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(report, schema)
        jsonschema.validate(cpp_report, schema)
        jsonschema.validate(template_report, schema)
        jsonschema.validate(declined, schema)
        jsonschema.validate(ambiguous, schema)
        jsonschema.validate(macro_ambiguous, schema)
        jsonschema.validate(no_success, schema)
        gcc_schema = json.loads(
            (ROOT / "schemas/whyvec-gcc-observation-report.schema.json").read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator.check_schema(gcc_schema)
        jsonschema.validate(gcc_report, gcc_schema)
        obligation_schema = json.loads(
            (ROOT / "schemas/whyvec-obligation-report.schema.json").read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator.check_schema(obligation_schema)
        jsonschema.validate(obligation, obligation_schema)
        jsonschema.validate(volatile_obligation, obligation_schema)
        agent_schema = json.loads(
            (ROOT / "schemas/whyvec-agent-trace.schema.json").read_text(encoding="utf-8")
        )
        jsonschema.Draft202012Validator.check_schema(agent_schema)
        jsonschema.validate(trace, agent_schema)
        jsonschema.validate(mismatched_trace, agent_schema)
        jsonschema.validate(refusal_trace, agent_schema)

        obligation_tamper_report = tamper_copy(
            obligation, temporary_path / "tampered-obligation"
        )
        obligation_artifact = obligation["artifacts"][0]
        obligation_artifact_path = (
            obligation_tamper_report.parent / obligation_artifact["path"]
        )
        obligation_artifact_path.chmod(
            obligation_artifact_path.stat().st_mode | stat.S_IWUSR
        )
        obligation_artifact_path.write_bytes(
            obligation_artifact_path.read_bytes() + b"tampered\n"
        )
        obligation_rejected = subprocess.run(
            [str(binary), "replay-obligation", str(obligation_tamper_report)],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if (
            obligation_rejected.returncode == 0
            or "digest or size mismatch" not in obligation_rejected.stderr
        ):
            raise RuntimeError(
                "obligation replay accepted a modified artifact: "
                f"{obligation_rejected.stderr}"
            )

        gcc_tamper_report = tamper_copy(gcc_report, temporary_path / "tampered-gcc")
        gcc_artifact = gcc_report["artifacts"][0]
        gcc_artifact_path = (
            gcc_tamper_report.parent / gcc_artifact["path"]
        )
        gcc_artifact_path.chmod(gcc_artifact_path.stat().st_mode | stat.S_IWUSR)
        gcc_artifact_path.write_bytes(gcc_artifact_path.read_bytes() + b"tampered\n")
        gcc_rejected = subprocess.run(
            [str(binary), "replay-gcc-opt", str(gcc_tamper_report)],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if gcc_rejected.returncode == 0 or "digest or size mismatch" not in gcc_rejected.stderr:
            raise RuntimeError(
                f"GCC replay accepted a modified artifact: {gcc_rejected.stderr}"
            )

        declined_path = tamper_copy(declined, temporary_path / "tampered-decline")
        declined_path.chmod(declined_path.stat().st_mode | stat.S_IWUSR)
        altered_report = json.loads(declined_path.read_text(encoding="utf-8"))
        altered_report["caveats"].append("unrecorded semantic change")
        declined_path.write_text(json.dumps(altered_report), encoding="utf-8")
        report_rejected = subprocess.run(
            [str(binary), "replay-opt", str(declined_path)],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if (
            report_rejected.returncode == 0
            or "report.json semantic contents" not in report_rejected.stderr
        ):
            raise RuntimeError(
                "optimization replay accepted modified report semantics: "
                f"{report_rejected.stderr}"
            )

        optimization_tamper_report = tamper_copy(
            report, temporary_path / "tampered-optimization"
        )
        artifact = report["artifacts"][0]
        artifact_path = optimization_tamper_report.parent / artifact["path"]
        artifact_path.chmod(artifact_path.stat().st_mode | stat.S_IWUSR)
        artifact_path.write_bytes(artifact_path.read_bytes() + b"tampered\n")
        rejected = subprocess.run(
            [str(binary), "replay-opt", str(optimization_tamper_report)],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if rejected.returncode == 0 or "digest or size mismatch" not in rejected.stderr:
            raise RuntimeError(
                f"optimization replay accepted a modified artifact: {rejected.stderr}"
            )

        if bundle_root:
            shutil.rmtree(repository / ".git")
            retained_opt_replay = json.loads(
                run([str(binary), "replay-opt", str(report["artifact_path"])]).stdout
            )
            retained_obligation_replay = json.loads(
                run(
                    [
                        str(binary),
                        "replay-obligation",
                        str(obligation["artifact_path"]),
                    ]
                ).stdout
            )
            if (
                retained_opt_replay.get("matched") is not True
                or retained_obligation_replay.get("matched") is not True
            ):
                raise RuntimeError("retained agent bundle did not replay after Git removal")
            replay_record = bundle_root / "replay.json"
            replay_record.write_text(
                json.dumps(
                    {
                        "optimization": retained_opt_replay,
                        "obligation": retained_obligation_replay,
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            replay_record.chmod(0o444)
            retained_ids = {
                report["analysis_id"],
                obligation["analysis_id"],
                retained_opt_replay["replay_analysis_id"],
                retained_obligation_replay["replay_analysis_id"],
            }
            analyses_root = repository / ".whyvec/analyses"
            for analysis in analyses_root.iterdir():
                if not analysis.is_dir() or analysis.name in retained_ids:
                    continue
                for path in sorted(analysis.rglob("*"), reverse=True):
                    path.chmod(0o755 if path.is_dir() else 0o644)
                analysis.chmod(0o755)
                shutil.rmtree(analysis)

    print("optimization-causality CLI validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
