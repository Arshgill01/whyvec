#!/usr/bin/env python3
"""Build the pinned LLVM helpers next to the development WhyVec binary."""

from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "target" / "debug"


def tool(name: str) -> str:
    resolved = shutil.which(name)
    if resolved is None:
        raise RuntimeError(f"required tool is unavailable: {name}")
    return resolved


def llvm_flags(*libraries: str) -> list[str]:
    completed = subprocess.run(
        [
            tool("llvm-config-21"),
            "--cxxflags",
            "--ldflags",
            "--system-libs",
            "--libs",
            *libraries,
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.split()


def build(destination: Path, source: str, output: str, *libraries: str) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            tool("clang++-21"),
            "-std=c++17",
            str(ROOT / source),
            *llvm_flags(*libraries),
            "-o",
            str(destination / output),
        ],
        cwd=ROOT,
        check=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=OUTPUT)
    arguments = parser.parse_args()
    destination = arguments.output.resolve()
    build(
        destination,
        "tools/whyvec-llvm-transform.cpp",
        "whyvec-llvm-transform",
        "core",
        "irreader",
        "bitwriter",
        "support",
    )
    build(
        destination,
        "tools/whyvec-llvm-loop-identity.cpp",
        "whyvec-llvm-loop-identity",
        "core",
        "irreader",
        "analysis",
        "support",
    )
    print(f"built WhyVec helpers in {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
