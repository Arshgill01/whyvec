#!/usr/bin/env python3
"""Validate stable and ambiguous LLVM loop identity behavior."""

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
        command, cwd=ROOT, check=False, capture_output=True, text=True, timeout=60
    )
    if (completed.returncode == 0) != success:
        raise RuntimeError(
            f"unexpected command result {completed.returncode}: {' '.join(command)}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def llvm_flags() -> list[str]:
    return run(
        [
            tool("llvm-config-21"),
            "--cxxflags",
            "--ldflags",
            "--system-libs",
            "--libs",
            "core",
            "irreader",
            "analysis",
            "support",
        ]
    ).stdout.split()


def compile_helper(source: str, output: Path) -> None:
    run(
        [
            tool("clang++-21"),
            "-std=c++17",
            str(ROOT / source),
            *llvm_flags(),
            "-o",
            str(output),
        ]
    )


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="whyvec-loop-identity-") as temporary:
        output = Path(temporary)
        inspector = output / "inspect"
        transformer = output / "transform"
        compile_helper("tools/whyvec-llvm-loop-identity.cpp", inspector)
        compile_helper("tools/whyvec-llvm-transform.cpp", transformer)

        source = ROOT / "fixtures/cases/bound-alias/kernel.c"
        baseline = output / "baseline.ll"
        run(
            [
                tool("clang-21"),
                "-O3",
                "-gline-tables-only",
                "-gcolumn-info",
                "-Xclang",
                "-disable-llvm-passes",
                "-emit-llvm",
                "-S",
                str(source),
                "-o",
                str(baseline),
            ]
        )
        first = json.loads(
            run(
                [str(inspector), str(baseline), "--function", "add_vectors_", "--line", "5"]
            ).stdout
        )
        variant = output / "variant.bc"
        run(
            [
                str(transformer),
                str(baseline),
                "--output",
                str(variant),
                "--function",
                "add_vectors_",
                "--parameter-index",
                "2",
            ]
        )
        second = json.loads(
            run(
                [str(inspector), str(variant), "--function", "add_vectors_", "--line", "5"]
            ).stdout
        )
        if first != second or first.get("mapping_confidence") != "high":
            raise RuntimeError(f"loop identity drifted across typed variant: {first} {second}")

        absent = run(
            [str(inspector), str(baseline), "--function", "add_vectors_", "--line", "999"],
            success=False,
        )
        if json.loads(absent.stderr).get("code") != "identity.loop_absent":
            raise RuntimeError("absent loop was not declined")

        ambiguous_ir = output / "ambiguous.ll"
        ambiguous_ir.write_text(
            """source_filename = "ambiguous.c"
define void @ambiguous(ptr %p, i32 %n) !dbg !4 {
entry:
  br label %a, !dbg !8
a:
  %i = phi i32 [ 0, %entry ], [ %in, %a ]
  %in = add i32 %i, 1
  %ac = icmp slt i32 %in, %n
  br i1 %ac, label %a, label %b, !dbg !8
b:
  br label %c, !dbg !8
c:
  %j = phi i32 [ 0, %b ], [ %jn, %c ]
  %jn = add i32 %j, 1
  %cc = icmp slt i32 %jn, %n
  br i1 %cc, label %c, label %exit, !dbg !8
exit:
  ret void
}
!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!2, !3}
!0 = distinct !DICompileUnit(language: DW_LANG_C11, file: !1, producer: "whyvec", isOptimized: false, runtimeVersion: 0, emissionKind: LineTablesOnly)
!1 = !DIFile(filename: "ambiguous.c", directory: ".")
!2 = !{i32 2, !"Dwarf Version", i32 5}
!3 = !{i32 2, !"Debug Info Version", i32 3}
!4 = distinct !DISubprogram(name: "ambiguous", scope: !1, file: !1, line: 1, type: !5, scopeLine: 1, spFlags: DISPFlagDefinition, unit: !0)
!5 = !DISubroutineType(types: !6)
!6 = !{}
!8 = !DILocation(line: 2, column: 1, scope: !4)
""",
            encoding="utf-8",
        )
        ambiguous = run(
            [str(inspector), str(ambiguous_ir), "--function", "ambiguous", "--line", "2"],
            success=False,
        )
        decline = json.loads(ambiguous.stderr)
        if decline.get("code") != "identity.loop_ambiguous" or decline.get("matches") != 2:
            raise RuntimeError(f"ambiguous loops were not declined: {decline}")

    print("LLVM loop identity validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
