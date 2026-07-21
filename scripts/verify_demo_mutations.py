#!/usr/bin/env python3
"""Prove that the demo validation rejects unsafe guarded-candidate mutations."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)


def replace_last(source: str, before: str, after: str) -> str:
    head, separator, tail = source.rpartition(before)
    if not separator:
        raise RuntimeError(f"mutation anchor absent: {before}")
    return head + after + tail


def policy_violations(source: str) -> list[str]:
    violations = []
    signature = "void add_vectors_(int *output, const int *input, const int *count)"
    if source.count(signature) != 1:
        violations.append("changed_abi")
    if re.search(r"\brestrict\b", source):
        violations.append("unjustified_restrict")
    fallback = "for (int i = 0; i < *count; ++i) {\n    output[i] += input[i];\n  }"
    if fallback not in source:
        violations.append("original_fallback_changed")
    return violations


def execute_candidate(clang: str, source: str, demo: Path) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory(prefix="whyvec-demo-mutation-") as directory:
        root = Path(directory)
        (root / "src").mkdir()
        (root / "tests").mkdir()
        (root / "include").mkdir()
        (root / "src/kernel.c").write_text(source, encoding="utf-8")
        shutil.copy2(demo / "tests/test_differential.c", root / "tests/test_differential.c")
        shutil.copy2(demo / "include/whyvec_demo.h", root / "include/whyvec_demo.h")
        binary = root / "differential"
        compiled = run(
            [clang, "-std=c17", "-O2", "-I", str(root / "include"),
             str(root / "tests/test_differential.c"), "-o", str(binary)],
            root,
        )
        if compiled.returncode != 0:
            return compiled
        return run([str(binary)], root)


def expect_rejected(name: str, result: subprocess.CompletedProcess[str]) -> dict[str, object]:
    if result.returncode == 0:
        raise RuntimeError(f"mutation was not rejected: {name}")
    return {"mutation": name, "rejected": True, "exit_status": result.returncode}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--clang", default="clang-21")
    parser.add_argument("--demo", type=Path, default=ROOT / "demo")
    parser.add_argument("--candidate", type=Path)
    parser.add_argument(
        "--validation-report",
        type=Path,
        default=ROOT / "evidence/codex-live/2026-07-21/validation-report.json",
    )
    arguments = parser.parse_args()
    demo = arguments.demo.resolve(strict=True)
    candidate = (arguments.candidate or demo / "codex-generated/kernel.c").resolve(strict=True)
    source = candidate.read_text(encoding="utf-8")
    if policy_violations(source):
        raise RuntimeError("the retained Codex candidate fails the source policy")
    baseline = execute_candidate(arguments.clang, source, demo)
    if baseline.returncode != 0:
        raise RuntimeError(f"retained candidate failed differential validation: {baseline.stderr}")

    guard_return = "return output_end <= count_start || count_end <= output_start;"
    results = []
    results.append(expect_rejected(
        "always_true_guard",
        execute_candidate(arguments.clang, source.replace(guard_return, "return 1;"), demo),
    ))
    results.append(expect_rejected(
        "off_by_one_output_extent",
        execute_candidate(
            arguments.clang,
            source.replace(
                guard_return,
                "return output_end <= count_start || count_end <= output_start + sizeof(int);",
            ),
            demo,
        ),
    ))
    results.append(expect_rejected(
        "cached_bound_in_fallback",
        execute_candidate(
            arguments.clang,
            replace_last(source, "i < *count", "i < initial_count"),
            demo,
        ),
    ))
    results.append(expect_rejected(
        "altered_original_fallback",
        execute_candidate(
            arguments.clang,
            replace_last(source, "output[i] += input[i];", "output[i] -= input[i];"),
            demo,
        ),
    ))
    results.append(expect_rejected(
        "missing_fast_path_witness",
        execute_candidate(source=source.replace(
            "#define WHYVEC_FAST_PATH() (++whyvec_fast_paths)",
            "#define WHYVEC_FAST_PATH() ((void)0)",
        ), clang=arguments.clang, demo=demo),
    ))
    results.append(expect_rejected(
        "missing_fallback_witness",
        execute_candidate(source=source.replace(
            "#define WHYVEC_FALLBACK_PATH() (++whyvec_fallback_paths)",
            "#define WHYVEC_FALLBACK_PATH() ((void)0)",
        ), clang=arguments.clang, demo=demo),
    ))

    restrict_source = source.replace("int *output", "int *restrict output", 1)
    if "unjustified_restrict" not in policy_violations(restrict_source):
        raise RuntimeError("source policy accepted unjustified restrict")
    results.append({"mutation": "unjustified_restrict", "rejected": True})
    abi_source = source.replace("const int *count)", "const int *count, int mode)", 1)
    if "changed_abi" not in policy_violations(abi_source):
        raise RuntimeError("source policy accepted an ABI change")
    results.append({"mutation": "changed_abi", "rejected": True})

    with tempfile.TemporaryDirectory(prefix="whyvec-demo-evidence-mutation-") as directory:
        root = Path(directory)
        false_yaml = root / "false.yaml"
        false_yaml.write_text(
            "--- !Passed\nPass: loop-vectorize\nName: Vectorized\n"
            "DebugLoc: { File: kernel.c, Line: 1, Column: 1 }\n"
            "Function: add_vectors_\nArgs: []\n",
            encoding="utf-8",
        )
        record_result = run(
            ["python3", str(demo / "tools/assert_opt_records.py"), "--source", str(candidate),
             "--yaml", str(false_yaml), "--output", str(root / "records.json")],
            ROOT,
        )
        results.append(expect_rejected("falsified_compiler_record", record_result))

        csv_path = root / "noise.csv"
        with csv_path.open("w", newline="", encoding="utf-8") as destination:
            writer = csv.writer(destination)
            writer.writerow(["size", "sample", "order", "repetitions", "original_ns", "guarded_ns"])
            for size in [8, 31, 64, 257, 1024, 4096, 16384, 65536]:
                for sample in range(31):
                    writer.writerow([size, sample, "original-first", 1, 1000, 1000])
        noise_result = run(
            ["python3", str(demo / "tools/analyze_benchmark.py"), "--csv", str(csv_path),
             "--raw", str(root / "raw.json"), "--summary", str(root / "summary.json")],
            ROOT,
        )
        results.append(expect_rejected("benchmark_noise_as_improvement", noise_result))

    validation = json.loads(
        arguments.validation_report.read_text(encoding="utf-8")
    )
    mutated_digest = hashlib.sha256((source + "\n").encode()).hexdigest()
    if validation["candidate_source_sha256"] == mutated_digest:
        raise RuntimeError("candidate digest mutation did not change the digest")
    results.append({"mutation": "candidate_digest_mismatch", "rejected": True})

    print(json.dumps({"candidate_executions": json.loads(baseline.stdout), "mutations": results},
                     sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
