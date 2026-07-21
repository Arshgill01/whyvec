#!/usr/bin/env python3
"""Verify the pinned, independently sourced SuperLU SAXPY refusal case."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path


UPSTREAM = "https://github.com/xiaoyeli/superlu.git"
COMMIT = "a93143107e3854ba9716ee3d7ab40fca6880cc10"
SOURCE_URL = f"https://github.com/xiaoyeli/superlu/blob/{COMMIT}/CBLAS/saxpy.c"
LICENSE_URL = f"https://github.com/xiaoyeli/superlu/blob/{COMMIT}/License.txt"


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command, cwd=cwd, text=True, capture_output=True, check=False, timeout=300
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def redact(value: object, checkout: Path, whyvec_root: Path) -> object:
    if isinstance(value, str):
        return value.replace(str(checkout), "<checkout>").replace(
            str(whyvec_root), "<whyvec-repository>"
        )
    if isinstance(value, list):
        return [redact(item, checkout, whyvec_root) for item in value]
    if isinstance(value, dict):
        return {key: redact(item, checkout, whyvec_root) for key, item in value.items()}
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--whyvec", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--clang", default="clang-21")
    arguments = parser.parse_args()
    whyvec = arguments.whyvec.resolve(strict=True)
    whyvec_root = Path(__file__).resolve().parents[1]
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=False)

    with tempfile.TemporaryDirectory(prefix="whyvec-superlu-") as directory:
        checkout = Path(directory) / "superlu"
        run(["git", "clone", "--filter=blob:none", "--no-checkout", UPSTREAM, str(checkout)],
            whyvec_root)
        run(["git", "checkout", "--detach", COMMIT], checkout)
        observed_commit = run(["git", "rev-parse", "HEAD"], checkout).stdout.strip()
        if observed_commit != COMMIT:
            raise RuntimeError(f"upstream commit changed: {observed_commit}")
        license_text = (checkout / "License.txt").read_text(encoding="utf-8")
        if "Redistribution and use in source and binary forms" not in license_text:
            raise RuntimeError("expected permissive SuperLU license text is absent")

        build = checkout / "build"
        configure = [
            "cmake", "-S", str(checkout), "-B", str(build), "-G", "Ninja",
            "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
            "-Denable_internal_blaslib=ON", "-Denable_tests=ON", "-Denable_examples=OFF",
        ]
        environment = os.environ.copy()
        environment["CC"] = arguments.clang
        configured = subprocess.run(
            configure, cwd=checkout, env=environment, text=True, capture_output=True,
            check=False, timeout=300,
        )
        if configured.returncode != 0:
            raise RuntimeError(f"SuperLU configure failed: {configured.stdout}{configured.stderr}")
        run(["cmake", "--build", str(build), "--target", "blas", "s_test"], checkout)
        native_test = run(
            ["ctest", "--test-dir", str(build), "--output-on-failure", "-R", "^s_test_9_2_0_LA$"],
            checkout,
        )

        analyzed = run(
            [str(whyvec), "analyze", f"{checkout}/CBLAS/saxpy.c:72",
             "--repository", str(checkout), "--format", "json"],
            checkout,
        )
        result = json.loads(analyzed.stdout)
        optimization = result["optimization"]
        if optimization["monolithic_baseline"]["classification"] != "missed":
            raise RuntimeError("SuperLU SAXPY cleanup loop was not observed missed")
        decline = optimization.get("decline") or {}
        if decline.get("code") != "search.no_successful_assumption":
            raise RuntimeError(f"unexpected real-world result: {decline}")
        if optimization.get("finding") is not None or result.get("obligation") is not None:
            raise RuntimeError("unsupported real-world case produced a repair obligation")
        report_path = Path(optimization["artifact_path"])
        replay = json.loads(run([str(whyvec), "replay-opt", str(report_path)], checkout).stdout)
        if replay.get("matched") is not True:
            raise RuntimeError("real-world report replay did not match")

        report = json.loads(report_path.read_text(encoding="utf-8"))
        portable_report = redact(report, checkout, whyvec_root)
        (output / "analysis-report.json").write_text(
            json.dumps(portable_report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        summary = {
            "schema_version": "1.0.0",
            "case": "SuperLU CBLAS SAXPY cleanup loop",
            "upstream": UPSTREAM,
            "commit": COMMIT,
            "source_url": SOURCE_URL,
            "license_url": LICENSE_URL,
            "license": "BSD-3-Clause-style SuperLU license",
            "source": "CBLAS/saxpy.c:72",
            "build_configuration": [
                "CC=clang-21", "CMAKE_BUILD_TYPE=Release", "enable_internal_blaslib=ON",
                "enable_tests=ON", "enable_examples=OFF", "CMAKE_EXPORT_COMPILE_COMMANDS=ON",
            ],
            "repository_native_test": {
                "command": "ctest --test-dir <checkout>/build -R ^s_test_9_2_0_LA$",
                "passed": True,
                "output": native_test.stdout.replace(str(checkout), "<checkout>"),
            },
            "compiler_result": {
                "baseline": "observed missed",
                "counterfactual_result": "no successful assumption in the declared finite search",
                "decline_code": decline["code"],
                "minimality": optimization["minimality"],
                "structured_records": optimization["monolithic_baseline"]["structured_remarks"],
            },
            "decision": "principled_refusal",
            "decision_reason": (
                "WhyVec observed the real compiler miss but found no tested sufficient assumption; "
                "therefore it derived no source obligation and authorized no repair."
            ),
            "performance": "not measured because no repair was authorized",
            "replay": replay,
        }
        (output / "result.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print(json.dumps({"output": str(output), "decision": "principled_refusal"}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
