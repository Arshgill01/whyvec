#!/usr/bin/env python3
"""Verify canonical fixture classifications with the pinned Clang profile."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "toolchains/clang-21/profile.json"
MANIFEST_PATH = ROOT / "fixtures/manifest.json"


def fail(message: str) -> None:
    raise RuntimeError(message)


def select_clang() -> str:
    configured = os.environ.get("WHYVEC_CLANG")
    if configured:
        path = shutil.which(configured) if os.sep not in configured else configured
    else:
        path = shutil.which("clang-21")
    if not path or not Path(path).is_file():
        fail("Clang 21 is required; set WHYVEC_CLANG to the compiler binary")
    return str(Path(path).resolve())


def compile_fixture(
    clang: str, flags: list[str], source: Path, output: Path
) -> tuple[int, str]:
    command = [clang, *flags, "-c", str(source), "-o", str(output)]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    return completed.returncode, completed.stdout + completed.stderr


def main() -> int:
    profile = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    clang = select_clang()

    version = subprocess.run(
        [clang, "--version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout
    expected_version = str(profile["clang_version"])
    if expected_version not in version:
        fail(f"expected Clang {expected_version}, got: {version.splitlines()[0]}")

    if manifest["toolchain_profile"] != profile["profile_id"]:
        fail("fixture manifest and toolchain profile disagree")

    flags = [f"-{profile['optimization']}", f"-march={profile['cpu']}"]
    flags.extend(flag for flag in profile["required_flags"] if flag not in flags)

    with tempfile.TemporaryDirectory(prefix="whyvec-fixtures-") as temporary:
        output_root = Path(temporary)
        for case in manifest["cases"]:
            source = ROOT / "fixtures" / case["source"]
            exit_code, remarks = compile_fixture(
                clang, flags, source, output_root / f"{case['id']}.o"
            )
            if exit_code != 0:
                fail(f"{case['id']}: compilation failed\n{remarks}")

            selector_line = case["selector"]["line"]
            location = rf"{re.escape(str(source))}:{selector_line}:"
            if not re.search(location, remarks):
                fail(f"{case['id']}: no optimization record at selected loop line")

            expected = case["expected_baseline"]
            if expected == "vectorized" and "vectorized loop" not in remarks:
                fail(f"{case['id']}: expected vectorized baseline\n{remarks}")
            if expected == "missed" and "loop not vectorized" not in remarks:
                fail(f"{case['id']}: expected missed baseline\n{remarks}")
            if case["id"] == "bound-alias" and "uncountable loop" not in remarks:
                fail("bound-alias: expected pointer-loaded trip-count diagnostic")
            if case["id"] == "volatile-bound-refusal" and "volatile read" not in remarks:
                fail("volatile-bound-refusal: expected volatile-read diagnostic")

            print(f"{case['id']}: {expected}")

    print(f"fixture validation passed with Clang {expected_version}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, subprocess.SubprocessError, OSError, KeyError) as error:
        print(f"fixture validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
