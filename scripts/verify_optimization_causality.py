#!/usr/bin/env python3
"""Exercise the retained public optimization-causality query."""

from __future__ import annotations

import hashlib
import json
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
        jsonschema.validate(declined, schema)
        jsonschema.validate(no_success, schema)

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
