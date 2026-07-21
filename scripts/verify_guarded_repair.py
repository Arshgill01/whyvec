#!/usr/bin/env python3
"""Validate and retain the guarded bound-alias repair fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/cases/bound-alias-repair"


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def digest(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def retain(root: Path, relative: str, content: bytes, media_type: str) -> dict[str, object]:
    destination = root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise RuntimeError(f"refusing to overwrite retained artifact: {destination}")
    destination.write_bytes(content)
    return {
        "path": relative,
        "sha256": digest(content),
        "size": len(content),
        "media_type": media_type,
    }


def tool_identity(path: str) -> dict[str, str]:
    invocation = Path(shutil.which(path) or path).resolve(strict=True)
    resolved = invocation.resolve(strict=True)
    version = run([str(invocation), "--version"], ROOT).stdout.strip()
    return {
        "invocation_path": str(invocation),
        "resolved_path": str(resolved),
        "binary_digest": digest(resolved.read_bytes()),
        "version": version,
    }


def source_entry(path: Path) -> dict[str, str]:
    content = path.read_bytes()
    return {"path": str(path.relative_to(ROOT)), "sha256": digest(content)}


def normalize_command(command: list[str], build: Path) -> list[str]:
    normalized = []
    for argument in command:
        value = argument.replace(str(ROOT), "<repository>")
        value = value.replace(str(build), "<build>")
        normalized.append(value)
    return normalized


def median_and_mad(samples: list[int]) -> tuple[float, float]:
    median = float(statistics.median(samples))
    deviations = [abs(sample - median) for sample in samples]
    return median, float(statistics.median(deviations))


def environment() -> dict[str, object]:
    cpu_model = "unknown"
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("model name") and ":" in line:
                cpu_model = line.split(":", 1)[1].strip()
                break
    governor = "unknown"
    governor_path = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    if governor_path.is_file():
        governor = governor_path.read_text(encoding="utf-8").strip()
    affinity = sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else []
    return {
        "kernel": platform.release(),
        "machine": platform.machine(),
        "cpu_model": cpu_model,
        "governor": governor,
        "affinity": affinity,
        "python": platform.python_version(),
    }


def validate_obligation(path: Path) -> dict[str, str]:
    report = json.loads(path.read_text(encoding="utf-8"))
    obligation = report.get("obligation")
    if not isinstance(obligation, dict) or obligation.get("family") != (
        "bound_object_disjoint_from_modified_region"
    ):
        raise RuntimeError("guarded validation requires the positive bound-alias obligation")
    if report.get("decline") is not None:
        raise RuntimeError("guarded validation cannot consume a declined obligation")
    return {
        "analysis_id": report["analysis_id"],
        "semantic_digest": report["semantic_digest"],
        "family": obligation["family"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--obligation-report", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--clang", default="clang-21")
    arguments = parser.parse_args()

    obligation = validate_obligation(arguments.obligation_report)
    clang = tool_identity(arguments.clang)
    artifact_root = arguments.artifact_root.resolve()
    artifact_root.mkdir(parents=True, exist_ok=False)
    artifacts: list[dict[str, object]] = []
    commands: list[list[str]] = []

    with tempfile.TemporaryDirectory(prefix="whyvec-guarded-repair-") as directory:
        build = Path(directory)
        sources = [
            FIXTURE / "original.c",
            FIXTURE / "guarded.c",
            FIXTURE / "harness.c",
        ]
        differential_binary = build / "differential"
        differential_command = [
            clang["invocation_path"],
            "-std=c17",
            "-O2",
            "-march=x86-64-v3",
            *map(str, sources),
            "-o",
            str(differential_binary),
        ]
        commands.append(differential_command)
        run(differential_command, ROOT)
        differential = run([str(differential_binary)], ROOT)
        differential_result = json.loads(differential.stdout)
        expected = {
            "executions": 9,
            "fast_paths": 5,
            "fallback_paths": 4,
            "overflow_refusals": 2,
        }
        if differential_result != expected:
            raise RuntimeError(f"unexpected differential coverage: {differential_result}")
        artifacts.append(
            retain(
                artifact_root,
                "checks/differential.json",
                (json.dumps(differential_result, sort_keys=True) + "\n").encode(),
                "application/json",
            )
        )

        sanitizer_binary = build / "sanitizer"
        sanitizer_command = [
            clang["invocation_path"],
            "-std=c17",
            "-O1",
            "-g",
            "-fno-omit-frame-pointer",
            "-fsanitize=address,undefined",
            *map(str, sources),
            "-o",
            str(sanitizer_binary),
        ]
        commands.append(sanitizer_command)
        run(sanitizer_command, ROOT)
        sanitizer = run([str(sanitizer_binary)], ROOT)
        sanitizer_result = json.loads(sanitizer.stdout)
        if sanitizer_result != expected or sanitizer.stderr:
            raise RuntimeError(
                f"sanitizer validation was not clean: {sanitizer_result} {sanitizer.stderr}"
            )
        artifacts.append(
            retain(
                artifact_root,
                "checks/sanitizer.json",
                (json.dumps(sanitizer_result, sort_keys=True) + "\n").encode(),
                "application/json",
            )
        )

        optimization_record = build / "guarded.opt.yaml"
        optimization_command = [
            clang["invocation_path"],
            "-std=c17",
            "-O3",
            "-march=x86-64-v3",
            "-Rpass=loop-vectorize",
            "-Rpass-missed=loop-vectorize",
            "-Rpass-analysis=loop-vectorize",
            "-fsave-optimization-record=yaml",
            f"-foptimization-record-file={optimization_record}",
            "-c",
            str(FIXTURE / "guarded.c"),
            "-o",
            str(build / "guarded.o"),
        ]
        commands.append(optimization_command)
        optimization = run(optimization_command, ROOT)
        if "guarded.c:38:5: remark: vectorized loop" not in optimization.stderr:
            raise RuntimeError("fast path did not emit the expected vectorization record")
        if "guarded.c:43:3: remark: loop not vectorized" not in optimization.stderr:
            raise RuntimeError("unchanged fallback miss was not retained")
        artifacts.extend(
            [
                retain(
                    artifact_root,
                    "compiler/optimization.opt.yaml",
                    optimization_record.read_bytes(),
                    "application/yaml",
                ),
                retain(
                    artifact_root,
                    "compiler/remarks.txt",
                    optimization.stderr.encode(),
                    "text/plain",
                ),
            ]
        )

        benchmark_binary = build / "benchmark"
        benchmark_command = [
            clang["invocation_path"],
            "-std=c17",
            "-O3",
            "-march=x86-64-v3",
            str(FIXTURE / "original.c"),
            str(FIXTURE / "guarded.c"),
            str(FIXTURE / "benchmark.c"),
            "-o",
            str(benchmark_binary),
        ]
        commands.append(benchmark_command)
        run(benchmark_command, ROOT)
        benchmark = json.loads(run([str(benchmark_binary)], ROOT).stdout)
        original_median, original_mad = median_and_mad(benchmark["original_ns"])
        guarded_median, guarded_mad = median_and_mad(benchmark["guarded_ns"])
        ratio = original_median / guarded_median
        separation = original_median - guarded_median
        noise = 3.0 * (original_mad + guarded_mad)
        classification = (
            "measured_improvement"
            if separation > noise and guarded_median < original_median
            else "noise_decline"
        )
        benchmark_summary = {
            "classification": classification,
            "original_median_ns": original_median,
            "original_mad_ns": original_mad,
            "guarded_median_ns": guarded_median,
            "guarded_mad_ns": guarded_mad,
            "median_ratio": ratio,
            "decision_rule": "improvement only when median separation exceeds three times summed MAD",
        }
        artifacts.extend(
            [
                retain(
                    artifact_root,
                    "benchmark/raw.json",
                    (json.dumps(benchmark, sort_keys=True) + "\n").encode(),
                    "application/json",
                ),
                retain(
                    artifact_root,
                    "benchmark/summary.json",
                    (json.dumps(benchmark_summary, sort_keys=True) + "\n").encode(),
                    "application/json",
                ),
            ]
        )

    sources = [
        source_entry(FIXTURE / name)
        for name in ("original.c", "guarded.c", "harness.c", "benchmark.c")
    ]
    environment_record = environment()
    normalized_commands = [normalize_command(command, build) for command in commands]
    artifacts.append(
        retain(
            artifact_root,
            "environment.json",
            (json.dumps(environment_record, sort_keys=True) + "\n").encode(),
            "application/json",
        )
    )
    material = json.dumps(
        {"obligation": obligation, "sources": sources, "clang": clang}, sort_keys=True
    ).encode()
    analysis_id = f"wv_{digest(material + str(time.time_ns()).encode())[:24]}"
    report = {
        "schema_version": "1.0.0",
        "analysis_id": analysis_id,
        "query_kind": "repair_validation",
        "evidence_strength": "validated_on_covered_executions",
        "obligation": obligation,
        "toolchain": {"clang": clang, "flags": ["-march=x86-64-v3"]},
        "sources": sources,
        "commands": normalized_commands,
        "differential": differential_result,
        "sanitizer": {"clean": True, "covered": sanitizer_result},
        "optimization": {
            "fast_path": "vectorized",
            "fallback": "missed",
            "fast_path_line": 38,
            "fallback_line": 43,
        },
        "benchmark": {"raw": benchmark, "summary": benchmark_summary},
        "environment": environment_record,
        "artifacts": artifacts,
        "artifact_path": str(
            (artifact_root / "report.json").relative_to(ROOT)
            if (artifact_root / "report.json").is_relative_to(ROOT)
            else artifact_root / "report.json"
        ),
        "caveats": [
            "Differential agreement is validated on covered executions, not full semantic equivalence.",
            "The uintptr_t guard is limited to the recorded flat x86-64 target policy.",
            "Benchmark classification applies only to the retained environment and workload.",
        ],
    }
    report_content = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode()
    (artifact_root / "report.json").write_bytes(report_content)

    schema = json.loads(
        (ROOT / "schemas/whyvec-validation-report.schema.json").read_text(encoding="utf-8")
    )
    try:
        import jsonschema
    except ImportError as error:
        raise RuntimeError("jsonschema is required") from error
    jsonschema.Draft202012Validator(schema).validate(report)

    for path in sorted(artifact_root.rglob("*"), reverse=True):
        if path.is_file():
            path.chmod(0o444)
    artifact_root.chmod(0o555)
    print(json.dumps({"report": str(artifact_root / "report.json"), **benchmark_summary}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
