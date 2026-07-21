#!/usr/bin/env python3
"""Build and adversarially validate the typed LLVM noalias transformer."""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def tool(name: str) -> str:
    resolved = shutil.which(name)
    if not resolved:
        raise RuntimeError(f"required tool is unavailable: {name}")
    return resolved


def run(command: list[str], *, success: bool = True) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if (completed.returncode == 0) != success:
        raise RuntimeError(
            f"unexpected command result {completed.returncode}: {' '.join(command)}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def llvm_flags() -> list[str]:
    output = run(
        [
            tool("llvm-config-21"),
            "--cxxflags",
            "--ldflags",
            "--system-libs",
            "--libs",
            "core",
            "irreader",
            "bitwriter",
            "support",
        ]
    ).stdout
    return output.split()


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="whyvec-llvm-transform-") as temporary:
        output = Path(temporary)
        binary = output / "whyvec-llvm-transform"
        run(
            [
                tool("clang++-21"),
                "-std=c++17",
                str(ROOT / "tools/whyvec-llvm-transform.cpp"),
                *llvm_flags(),
                "-o",
                str(binary),
            ]
        )
        source = ROOT / "fixtures/cases/bound-alias/kernel.c"
        baseline = output / "baseline.ll"
        run(
            [
                tool("clang-21"),
                "-O3",
                "-march=x86-64-v3",
                "-gline-tables-only",
                "-Xclang",
                "-disable-llvm-passes",
                "-emit-llvm",
                "-S",
                str(source),
                "-o",
                str(baseline),
            ]
        )

        transformed = output / "count-noalias.bc"
        applied = run(
            [
                str(binary),
                str(baseline),
                "--output",
                str(transformed),
                "--function",
                "add_vectors_",
                "--parameter-index",
                "2",
            ]
        )
        result = json.loads(applied.stdout)
        if result.get("verifier") != "passed" or result.get("after") is not True:
            raise RuntimeError(f"unexpected transformer result: {result}")
        disassembled = run([tool("llvm-dis-21"), str(transformed), "-o", "-"]).stdout
        signature = next(
            line for line in disassembled.splitlines() if line.startswith("define")
        )
        if "ptr noalias" not in signature:
            raise RuntimeError(f"typed noalias attribute is absent: {signature}")
        baseline_bitcode = output / "baseline.bc"
        run([tool("llvm-as-21"), str(baseline), "-o", str(baseline_bitcode)])
        canonical_baseline = run(
            [tool("llvm-dis-21"), str(baseline_bitcode), "-o", "-"]
        ).stdout
        if disassembled.count(" noalias") != canonical_baseline.count(" noalias") + 1:
            raise RuntimeError("transformed module has an unexpected noalias delta count")
        normalized_variant = "\n".join(disassembled.splitlines()[1:])
        normalized_baseline = "\n".join(canonical_baseline.splitlines()[1:])
        if normalized_variant.replace(" noalias", "", 1) != normalized_baseline:
            raise RuntimeError("transformed module changed outside the declared attribute")

        rerun = run(
            [
                str(binary),
                str(transformed),
                "--output",
                str(output / "invalid.bc"),
                "--function",
                "add_vectors_",
                "--parameter-index",
                "2",
            ],
            success=False,
        )
        if json.loads(rerun.stderr).get("code") != "variant.assumption_already_present":
            raise RuntimeError("existing noalias was not declined")

        mixed = output / "mixed.ll"
        mixed.write_text(
            "define void @mixed(ptr %pointer, i32 %value) { ret void }\n",
            encoding="utf-8",
        )
        non_pointer = run(
            [
                str(binary),
                str(mixed),
                "--output",
                str(output / "non-pointer.bc"),
                "--function",
                "mixed",
                "--parameter-index",
                "1",
            ],
            success=False,
        )
        if json.loads(non_pointer.stderr).get("code") != "variant.parameter_not_pointer":
            raise RuntimeError("non-pointer intervention was not declined")

        absent = run(
            [
                str(binary),
                str(baseline),
                "--output",
                str(output / "absent.bc"),
                "--function",
                "not_a_function",
                "--parameter-index",
                "0",
            ],
            success=False,
        )
        if json.loads(absent.stderr).get("code") != "identity.function_absent":
            raise RuntimeError("absent function was not declined")

        run([tool("opt-21"), "-passes=verify", str(transformed), "-o", "/dev/null"])

    print("typed LLVM transformer validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
