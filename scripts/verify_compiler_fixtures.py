#!/usr/bin/env python3
"""Execute cross-frontend baseline and LLVM counterfactual fixture checks."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "fixtures/manifest.json"
TOOLCHAINS = ROOT / "toolchains"


class VerificationError(RuntimeError):
    """Raised when executable compiler evidence violates a fixture contract."""


@dataclass(frozen=True)
class Toolchain:
    frontend: str
    frontend_path: Path
    optimizer_path: Path
    profile: dict[str, object]


def require_tool(config_name: str, default_name: str) -> Path:
    configured = os.environ.get(config_name, default_name)
    resolved = shutil.which(configured) if os.sep not in configured else configured
    if not resolved or not Path(resolved).is_file():
        raise VerificationError(
            f"required tool {default_name!r} is unavailable; set {config_name}"
        )
    # Keep the invocation path intact. Toolchain proxies such as rustup select
    # the delegated compiler from argv[0]; resolving the symlink would execute
    # the proxy as `rustup` instead of `rustc`. Production fingerprints retain
    # both this invocation path and the resolved binary identity.
    return Path(resolved).absolute()


def run(command: list[str], *, timeout: int = 45) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if completed.returncode != 0:
        rendered = " ".join(command)
        raise VerificationError(
            f"command failed ({completed.returncode}): {rendered}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def load_profiles() -> dict[str, dict[str, object]]:
    profiles: dict[str, dict[str, object]] = {}
    for path in sorted(TOOLCHAINS.glob("*/profile.json")):
        profile = json.loads(path.read_text(encoding="utf-8"))
        profile_id = profile.get("profile_id")
        if not isinstance(profile_id, str):
            raise VerificationError(f"{path}: profile_id is missing")
        if profile_id in profiles:
            raise VerificationError(f"duplicate toolchain profile: {profile_id}")
        profiles[profile_id] = profile
    return profiles


def resolve_toolchain(profile: dict[str, object]) -> Toolchain:
    profile_id = str(profile["profile_id"])
    if profile_id.startswith("clang-"):
        return Toolchain(
            frontend="clang",
            frontend_path=require_tool("WHYVEC_CLANG_21", "clang-21"),
            optimizer_path=require_tool("WHYVEC_OPT_21", "opt-21"),
            profile=profile,
        )
    if profile_id.startswith("rustc-"):
        return Toolchain(
            frontend="rustc",
            frontend_path=require_tool("WHYVEC_RUSTC", "rustc"),
            optimizer_path=require_tool("WHYVEC_OPT_22", "opt-22"),
            profile=profile,
        )
    raise VerificationError(f"unsupported fixture profile: {profile_id}")


def assert_tool_versions(toolchain: Toolchain) -> None:
    frontend_version = run([str(toolchain.frontend_path), "--version"]).stdout
    optimizer_version = run([str(toolchain.optimizer_path), "--version"]).stdout
    profile = toolchain.profile

    if toolchain.frontend == "clang":
        expected_frontend = str(profile["clang_version"])
    else:
        expected_frontend = str(profile["rustc_version"])
    expected_optimizer = str(profile["llvm_version"])

    if expected_frontend not in frontend_version:
        raise VerificationError(
            f"expected {toolchain.frontend} {expected_frontend}, got "
            f"{frontend_version.splitlines()[0]}"
        )
    if expected_optimizer not in optimizer_version:
        raise VerificationError(
            f"expected LLVM {expected_optimizer}, got {optimizer_version.splitlines()[0]}"
        )


def optimization_remark_flags() -> list[str]:
    return [
        "-pass-remarks=loop-vectorize",
        "-pass-remarks-missed=loop-vectorize",
        "-pass-remarks-analysis=loop-vectorize",
    ]


def classify_remarks(remarks: str) -> str:
    if "vectorized loop" in remarks:
        return "vectorized"
    if "loop not vectorized" in remarks:
        return "missed"
    raise VerificationError(f"no loop-vectorization observation found:\n{remarks}")


def assert_selected_location(case: dict[str, object], source: Path, remarks: str) -> None:
    selector = case["selector"]
    if not isinstance(selector, dict):
        raise VerificationError(f"{case['id']}: invalid selector")
    line = int(selector["line"])
    absolute_location = f"{source}:{line}:"
    relative_location = f"{source.relative_to(ROOT)}:{line}:"
    if absolute_location not in remarks and relative_location not in remarks:
        raise VerificationError(
            f"{case['id']}: no observation at selected line {line}\n{remarks}"
        )


def compile_clang_baseline(
    toolchain: Toolchain, source: Path, output: Path
) -> str:
    profile = toolchain.profile
    command = [
        str(toolchain.frontend_path),
        f"-{profile['optimization']}",
        f"-march={profile['cpu']}",
        "-gline-tables-only",
        "-gcolumn-info",
        "-Rpass=loop-vectorize",
        "-Rpass-missed=loop-vectorize",
        "-Rpass-analysis=loop-vectorize",
        "-c",
        str(source),
        "-o",
        str(output),
    ]
    completed = run(command)
    return completed.stdout + completed.stderr


def emit_clang_preopt_ir(
    toolchain: Toolchain, source: Path, output: Path
) -> None:
    profile = toolchain.profile
    run(
        [
            str(toolchain.frontend_path),
            f"-{profile['optimization']}",
            f"-march={profile['cpu']}",
            "-gline-tables-only",
            "-gcolumn-info",
            "-Xclang",
            "-disable-llvm-passes",
            "-emit-llvm",
            "-S",
            str(source),
            "-o",
            str(output),
        ]
    )


def rust_common_flags(profile: dict[str, object]) -> list[str]:
    return [
        "--crate-name=whyvec_fixture",
        "--crate-type=lib",
        "--edition=2024",
        "-O",
        "-Ccodegen-units=1",
        "-Cdebuginfo=1",
        f"-Ctarget-cpu={profile['cpu']}",
    ]


def compile_rust_baseline(
    toolchain: Toolchain, source: Path, output: Path
) -> str:
    command = [
        str(toolchain.frontend_path),
        *rust_common_flags(toolchain.profile),
        "-Cremark=loop-vectorize",
        f"--emit=obj={output}",
        str(source),
    ]
    completed = run(command)
    return completed.stdout + completed.stderr


def emit_rust_preopt_ir(toolchain: Toolchain, source: Path, output: Path) -> None:
    run(
        [
            str(toolchain.frontend_path),
            *rust_common_flags(toolchain.profile),
            "-Cno-prepopulate-passes",
            f"--emit=llvm-ir={output}",
            str(source),
        ]
    )


def capture_clang_pipeline(toolchain: Toolchain, source: Path, output: Path) -> str:
    profile = toolchain.profile
    completed = run(
        [
            str(toolchain.frontend_path),
            f"-{profile['optimization']}",
            f"-march={profile['cpu']}",
            "-mllvm",
            "-print-pipeline-passes",
            "-c",
            str(source),
            "-o",
            str(output),
        ]
    )
    pipeline = completed.stdout.strip()
    if not pipeline or "loop-vectorize" not in pipeline:
        raise VerificationError("Clang did not emit a replayable vectorization pipeline")
    return pipeline


def optimize_ir(
    toolchain: Toolchain, source: Path, output: Path, pipeline: str
) -> str:
    completed = run(
        [
            str(toolchain.optimizer_path),
            f"-passes={pipeline}",
            *optimization_remark_flags(),
            "-S",
            str(source),
            "-o",
            str(output),
        ]
    )
    return completed.stdout + completed.stderr


def find_function_arguments(ir: str, function: str) -> tuple[int, int, list[tuple[int, int]]]:
    marker = re.compile(rf"(?m)^define\b[^\n]*@{re.escape(function)}\(")
    match = marker.search(ir)
    if match is None:
        raise VerificationError(f"function {function!r} is absent from emitted IR")

    arguments_start = match.end()
    depth = 0
    segment_start = arguments_start
    segments: list[tuple[int, int]] = []
    position = arguments_start
    while position < len(ir):
        character = ir[position]
        if character in "([{<":
            depth += 1
        elif character in ")]}>" and depth > 0:
            depth -= 1
        elif character == "," and depth == 0:
            segments.append((segment_start, position))
            segment_start = position + 1
        elif character == ")" and depth == 0:
            segments.append((segment_start, position))
            return arguments_start, position, segments
        position += 1
    raise VerificationError(f"unterminated function signature for {function!r}")


def add_parameter_noalias(ir: str, function: str, parameter_index: int) -> str:
    _, _, segments = find_function_arguments(ir, function)
    if parameter_index >= len(segments):
        raise VerificationError(
            f"function {function!r} has no IR parameter {parameter_index}"
        )
    start, end = segments[parameter_index]
    segment = ir[start:end]
    if " noalias " in f" {segment} ":
        raise VerificationError(
            f"function {function!r} parameter {parameter_index} is already noalias"
        )
    pointer = re.search(r"\bptr\b(?:\s+addrspace\(\d+\))?", segment)
    if pointer is None:
        raise VerificationError(
            f"function {function!r} parameter {parameter_index} is not an opaque pointer"
        )
    insertion = pointer.end()
    changed_segment = segment[:insertion] + " noalias" + segment[insertion:]
    changed = ir[:start] + changed_segment + ir[end:]
    if len(changed) - len(ir) != len(" noalias"):
        raise VerificationError("noalias intervention changed an unexpected byte count")
    return changed


def verify_counterfactuals(
    case: dict[str, object],
    toolchain: Toolchain,
    preopt_ir: Path,
    output_root: Path,
    pipeline: str,
    transformer: Path,
) -> None:
    expected_values = case.get("expected_successful_singletons")
    if not isinstance(expected_values, list):
        return
    expected = {str(value) for value in expected_values}
    parameters = case.get("ir_parameters")
    selector = case.get("selector")
    if not isinstance(parameters, dict) or not isinstance(selector, dict):
        raise VerificationError(f"{case['id']}: counterfactual metadata is incomplete")

    baseline_remarks = optimize_ir(
        toolchain,
        preopt_ir,
        output_root / f"{case['id']}.split-baseline.opt.ll",
        pipeline,
    )
    if classify_remarks(baseline_remarks) != "missed":
        raise VerificationError(f"{case['id']}: split baseline must remain scalar")

    original_ir = preopt_ir.read_text(encoding="utf-8")
    actual: set[str] = set()
    function = str(selector["function"])
    for parameter_name, raw_index in sorted(parameters.items()):
        intervention_id = f"parameter.{parameter_name}.noalias"
        variant = output_root / f"{case['id']}.{parameter_name}.noalias.pre.bc"
        if toolchain.frontend == "clang":
            run(
                [
                    str(transformer),
                    str(preopt_ir),
                    "--output",
                    str(variant),
                    "--function",
                    function,
                    "--parameter-index",
                    str(raw_index),
                ]
            )
        else:
            changed_ir = add_parameter_noalias(original_ir, function, int(raw_index))
            variant.write_text(changed_ir, encoding="utf-8")

        run([str(toolchain.optimizer_path), "-passes=verify", str(variant), "-o", os.devnull])
        remarks = optimize_ir(
            toolchain,
            variant,
            output_root / f"{case['id']}.{parameter_name}.noalias.opt.ll",
            pipeline,
        )
        if classify_remarks(remarks) == "vectorized":
            actual.add(intervention_id)

    if actual != expected:
        raise VerificationError(
            f"{case['id']}: successful singleton mismatch; "
            f"expected {sorted(expected)}, got {sorted(actual)}"
        )

    monolithic_values = case.get("monolithic_counterfactuals", {})
    if not isinstance(monolithic_values, dict):
        raise VerificationError(
            f"{case['id']}: monolithic_counterfactuals must be an object"
        )
    confirmed: set[str] = set()
    for intervention_id, source_value in sorted(monolithic_values.items()):
        source = ROOT / "fixtures" / str(source_value)
        if toolchain.frontend == "clang":
            remarks = compile_clang_baseline(
                toolchain,
                source,
                output_root / f"{case['id']}.{intervention_id}.monolithic.o",
            )
        elif toolchain.frontend == "rustc":
            remarks = compile_rust_baseline(
                toolchain,
                source,
                output_root / f"{case['id']}.{intervention_id}.monolithic.o",
            )
        else:
            raise VerificationError(f"{case['id']}: unsupported frontend")
        if classify_remarks(remarks) != "vectorized":
            raise VerificationError(
                f"{case['id']}: monolithic witness did not vectorize for "
                f"{intervention_id}\n{remarks}"
            )
        confirmed.add(str(intervention_id))

    if case.get("pipeline_fidelity") == "equivalent_confirmed":
        preferred = str(case.get("preferred_explanation_singleton", ""))
        if preferred not in confirmed or preferred not in actual:
            raise VerificationError(
                f"{case['id']}: equivalent_confirmed requires matching split and "
                f"monolithic outcomes for {preferred}"
            )


def verify_case(
    case: dict[str, object],
    toolchain: Toolchain,
    output_root: Path,
    transformer: Path,
) -> None:
    source = ROOT / "fixtures" / str(case["source"])
    case_id = str(case["id"])
    frontend_output = output_root / f"{case_id}.baseline.o"
    preopt_ir = output_root / f"{case_id}.pre.ll"

    if toolchain.frontend == "clang":
        remarks = compile_clang_baseline(toolchain, source, frontend_output)
        emit_clang_preopt_ir(toolchain, source, preopt_ir)
        pipeline = capture_clang_pipeline(
            toolchain, source, output_root / f"{case_id}.pipeline.o"
        )
    elif toolchain.frontend == "rustc":
        remarks = compile_rust_baseline(toolchain, source, frontend_output)
        emit_rust_preopt_ir(toolchain, source, preopt_ir)
        pipeline = "default<O3>"
    else:
        raise VerificationError(f"{case_id}: unsupported frontend")

    assert_selected_location(case, source, remarks)
    expected = str(case["expected_baseline"])
    actual = classify_remarks(remarks)
    if actual != expected:
        raise VerificationError(
            f"{case_id}: expected baseline {expected}, got {actual}\n{remarks}"
        )
    if expected == "missed" and case_id.endswith("bound-alias"):
        if "uncountable loop" not in remarks:
            raise VerificationError(
                f"{case_id}: missing pointer-loaded trip-count observation"
            )
    if case_id == "volatile-bound-refusal" and "volatile read" not in remarks:
        raise VerificationError(f"{case_id}: missing volatile-read observation")

    verify_counterfactuals(
        case, toolchain, preopt_ir, output_root, pipeline, transformer
    )
    fidelity = case.get("pipeline_fidelity", "not_applicable")
    print(
        f"{case_id}: frontend={toolchain.frontend} baseline={actual} "
        f"counterfactual_pipeline={fidelity}"
    )


def build_transformer(output_root: Path) -> Path:
    llvm_config = require_tool("WHYVEC_LLVM_CONFIG_21", "llvm-config-21")
    compiler = require_tool("WHYVEC_CLANGXX_21", "clang++-21")
    flags = run(
        [
            str(llvm_config),
            "--cxxflags",
            "--ldflags",
            "--system-libs",
            "--libs",
            "core",
            "irreader",
            "bitwriter",
            "support",
        ]
    ).stdout.split()
    transformer = output_root / "whyvec-llvm-transform"
    run(
        [
            str(compiler),
            "-std=c++17",
            str(ROOT / "tools/whyvec-llvm-transform.cpp"),
            *flags,
            "-o",
            str(transformer),
        ]
    )
    return transformer


def main() -> int:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    profiles = load_profiles()
    toolchains: dict[str, Toolchain] = {}

    for profile_id in manifest["toolchain_profiles"]:
        if profile_id not in profiles:
            raise VerificationError(f"fixture profile does not resolve: {profile_id}")
        toolchain = resolve_toolchain(profiles[profile_id])
        assert_tool_versions(toolchain)
        toolchains[profile_id] = toolchain

    with tempfile.TemporaryDirectory(prefix="whyvec-compiler-fixtures-") as temporary:
        output_root = Path(temporary)
        transformer = build_transformer(output_root)
        for case in manifest["cases"]:
            profile_id = str(case["toolchain_profile"])
            verify_case(case, toolchains[profile_id], output_root, transformer)

    print("cross-frontend compiler fixture validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (VerificationError, OSError, subprocess.SubprocessError, KeyError) as error:
        print(f"compiler fixture validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
