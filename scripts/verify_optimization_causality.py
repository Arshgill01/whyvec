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


def main() -> int:
    run(["cargo", "build", "--quiet", "-p", "whyvec-cli"])
    binary = ROOT / "target/debug/whyvec"
    with tempfile.TemporaryDirectory(prefix="whyvec-opt-cli-") as temporary:
        temporary_path = Path(temporary)
        repository = temporary_path / "repository"
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
        run(["git", "init", "--quiet"], repository)

        transformer = temporary_path / "whyvec-llvm-transform"
        identity = temporary_path / "whyvec-llvm-loop-identity"
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
        if report.get("minimality") != "minimal_in_declared_search":
            raise RuntimeError(f"unexpected non-unique minimality: {report.get('minimality')}")
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
            "executions": 9,
            "fast_paths": 5,
            "fallback_paths": 4,
            "overflow_refusals": 2,
        }:
            raise RuntimeError(f"guarded branch coverage changed: {validation_report}")
        if validation_report.get("optimization") != {
            "fast_path": "vectorized",
            "fallback": "missed",
            "fast_path_line": 38,
            "fallback_line": 43,
        }:
            raise RuntimeError("guarded compiler evidence changed")
        validate_artifacts(validation_report)

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

        obligation_artifact = obligation["artifacts"][0]
        obligation_artifact_path = (
            Path(str(obligation["artifact_path"])).parent / obligation_artifact["path"]
        )
        obligation_artifact_path.chmod(
            obligation_artifact_path.stat().st_mode | stat.S_IWUSR
        )
        obligation_artifact_path.write_bytes(
            obligation_artifact_path.read_bytes() + b"tampered\n"
        )
        obligation_rejected = subprocess.run(
            [str(binary), "replay-obligation", str(obligation["artifact_path"])],
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

        gcc_artifact = gcc_report["artifacts"][0]
        gcc_artifact_path = (
            Path(str(gcc_report["artifact_path"])).parent / gcc_artifact["path"]
        )
        gcc_artifact_path.chmod(gcc_artifact_path.stat().st_mode | stat.S_IWUSR)
        gcc_artifact_path.write_bytes(gcc_artifact_path.read_bytes() + b"tampered\n")
        gcc_rejected = subprocess.run(
            [str(binary), "replay-gcc-opt", str(gcc_report["artifact_path"])],
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

        declined_path = Path(str(declined["artifact_path"]))
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

        artifact = report["artifacts"][0]
        artifact_path = Path(str(report["artifact_path"])).parent / artifact["path"]
        artifact_path.chmod(artifact_path.stat().st_mode | stat.S_IWUSR)
        artifact_path.write_bytes(artifact_path.read_bytes() + b"tampered\n")
        rejected = subprocess.run(
            [str(binary), "replay-opt", str(report["artifact_path"])],
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

    print("optimization-causality CLI validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
