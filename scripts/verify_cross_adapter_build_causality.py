#!/usr/bin/env python3
"""Exercise GCC and TypeScript build-causality adapters through the public CLI."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads(
    (ROOT / "schemas/whyvec-build-report.schema.json").read_text(encoding="utf-8")
)


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


def initialize_git(repository: Path) -> None:
    run(["git", "init", "--quiet"], repository)
    run(["git", "config", "user.email", "whyvec@example.invalid"], repository)
    run(["git", "config", "user.name", "WhyVec Fixture"], repository)
    run(["git", "add", "."], repository)
    run(["git", "commit", "--quiet", "-m", "base"], repository)


def validate_report(
    report: dict[str, object], repository: Path, adapter: str, sufficient_file: str
) -> None:
    try:
        import jsonschema
    except ImportError as error:
        raise RuntimeError("jsonschema is required for report validation") from error

    jsonschema.Draft202012Validator(SCHEMA).validate(report)
    if report.get("adapter") != adapter:
        raise RuntimeError(f"unexpected adapter: {report.get('adapter')}")
    toolchain = report.get("toolchain")
    if not isinstance(toolchain, dict) or toolchain.get("adapter") != adapter:
        raise RuntimeError(f"toolchain adapter mismatch: {toolchain}")
    sandbox = toolchain.get("sandbox")
    if not isinstance(sandbox, dict) or not all(
        sandbox.get(field) is True
        for field in ("network_isolated", "host_root_read_only", "private_tmp")
    ):
        raise RuntimeError(f"sandbox guarantees are incomplete: {sandbox}")
    causal_sets = report.get("causal_sets")
    if not isinstance(causal_sets, list) or len(causal_sets) != 1:
        raise RuntimeError(f"expected one causal set: {causal_sets}")
    if causal_sets[0].get("sufficient_files") != [sufficient_file]:
        raise RuntimeError(f"unexpected sufficient files: {causal_sets[0]}")
    if causal_sets[0].get("target_removed_from_full_patch") is not True:
        raise RuntimeError("removal witness did not suppress the target")
    refinements = report.get("hunk_refinements")
    if not isinstance(refinements, list) or len(refinements) != 1:
        raise RuntimeError(f"expected one hunk refinement: {refinements}")
    if refinements[0].get("grouping") != "text_hunk_fallback":
        raise RuntimeError(f"non-Rust edit did not use explicit text fallback: {refinements}")
    artifact = report.get("artifact_path")
    if not isinstance(artifact, str) or not Path(artifact).is_file():
        raise RuntimeError("retained report is absent")
    if not Path(artifact).resolve().is_relative_to(
        repository.resolve() / ".whyvec" / "analyses"
    ):
        raise RuntimeError("retained report escaped the repository")


def invoke(
    binary: Path,
    repository: Path,
    diagnostic: str,
    source: str,
    command: list[str],
) -> dict[str, object]:
    completed = run(
        [
            str(binary),
            "explain-build",
            "--repository",
            str(repository),
            "--base",
            "HEAD",
            "--diagnostic",
            diagnostic,
            "--at",
            source,
            "--format",
            "json",
            "--",
            *command,
        ],
        ROOT,
    )
    report = json.loads(completed.stdout)
    identity = report["target_diagnostic"]["id"]
    identity_report = json.loads(
        run(
            [
                str(binary),
                "explain-build",
                "--repository",
                str(repository),
                "--base",
                "HEAD",
                "--diagnostic",
                identity,
                "--format",
                "json",
                "--",
                *command,
            ],
            ROOT,
        ).stdout
    )
    if identity_report["target_diagnostic"]["id"] != identity:
        raise RuntimeError("stable diagnostic identity selected a different observation")
    replay = json.loads(
        run([str(binary), "replay-build", report["artifact_path"]], ROOT).stdout
    )
    if replay.get("matched") is not True:
        raise RuntimeError(f"semantic replay did not match: {replay}")
    return report


def verify_typescript(binary: Path, temporary: Path) -> None:
    repository = temporary / "typescript"
    source = repository / "src"
    source.mkdir(parents=True)
    (repository / "tsconfig.json").write_text(
        json.dumps(
            {
                "compilerOptions": {
                    "noEmit": True,
                    "strict": True,
                    "target": "ES2022",
                    "module": "ESNext",
                },
                "include": ["src/**/*.ts"],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    (source / "api.ts").write_text(
        "export function measure(value: number): number { return value; }\n",
        encoding="utf-8",
    )
    (source / "consumer.ts").write_text(
        'import { measure } from "./api.js";\nexport const result = measure(7);\n',
        encoding="utf-8",
    )
    (source / "other.ts").write_text("export const label = 'base';\n", encoding="utf-8")
    initialize_git(repository)
    (source / "api.ts").write_text(
        "export function measure(value: string): number { return value.length; }\n",
        encoding="utf-8",
    )
    (source / "other.ts").write_text(
        "export const label = 'changed';\n", encoding="utf-8"
    )
    report = invoke(
        binary,
        repository,
        "TS2345",
        "src/consumer.ts",
        ["whyvec-typescript", "tsconfig.json"],
    )
    validate_report(report, repository, "typescript", "src/api.ts")
    toolchain = report["toolchain"]
    if len(toolchain.get("support_files", [])) != 2:
        raise RuntimeError("TypeScript adapter did not fingerprint its support files")


def verify_gcc(binary: Path, temporary: Path) -> None:
    repository = temporary / "gcc"
    source = repository / "src"
    source.mkdir(parents=True)
    (source / "api.hpp").write_text(
        "inline int measure(int value) { return value; }\n", encoding="utf-8"
    )
    (source / "main.cpp").write_text(
        '#include "api.hpp"\nint main() { return measure(7); }\n', encoding="utf-8"
    )
    (source / "other.hpp").write_text("#define LABEL 1\n", encoding="utf-8")
    initialize_git(repository)
    (source / "api.hpp").write_text(
        "inline int measure(const char *value) { return *value; }\n", encoding="utf-8"
    )
    (source / "other.hpp").write_text("#define LABEL 2\n", encoding="utf-8")
    report = invoke(
        binary,
        repository,
        "-fpermissive",
        "src/main.cpp",
        ["g++", "-std=c++20", "-fsyntax-only", "src/main.cpp"],
    )
    validate_report(report, repository, "gcc", "src/api.hpp")


def verify_clang(binary: Path, temporary: Path) -> None:
    repository = temporary / "clang"
    source = repository / "src"
    source.mkdir(parents=True)
    (source / "api.hpp").write_text(
        "inline int measure(int value) { return value; }\n", encoding="utf-8"
    )
    (source / "main.cpp").write_text(
        '#include "api.hpp"\nint main() { return measure(7); }\n', encoding="utf-8"
    )
    (source / "other.hpp").write_text("#define LABEL 1\n", encoding="utf-8")
    initialize_git(repository)
    (source / "api.hpp").write_text(
        "inline int measure(const char *value) { return *value; }\n", encoding="utf-8"
    )
    (source / "other.hpp").write_text("#define LABEL 2\n", encoding="utf-8")
    report = invoke(
        binary,
        repository,
        "4762",
        "src/main.cpp",
        ["clang++-21", "-std=c++20", "-fsyntax-only", "src/main.cpp"],
    )
    validate_report(report, repository, "clang", "src/api.hpp")


def main() -> int:
    adapter = ROOT / "tools/typescript-adapter"
    run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], adapter)
    run(["cargo", "build", "--quiet", "-p", "whyvec-cli"], ROOT)
    binary = ROOT / "target/debug/whyvec"
    with tempfile.TemporaryDirectory(prefix="whyvec-cross-build-") as directory:
        temporary = Path(directory)
        verify_typescript(binary, temporary)
        verify_gcc(binary, temporary)
        verify_clang(binary, temporary)
    print("cross-adapter build-causality validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
