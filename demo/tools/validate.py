#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path


def digest(content):
    return hashlib.sha256(content).hexdigest()


def run(name, command, outcomes, commands, *, cwd, stdout_path=None, env=None):
    completed = subprocess.run(command, cwd=cwd, env=env, capture_output=True, check=False)
    commands.append([str(item) for item in command])
    outcomes.append({"name": name, "command_index": len(commands) - 1,
                     "exit_status": completed.returncode,
                     "stdout_sha256": digest(completed.stdout), "stdout_size": len(completed.stdout),
                     "stderr_sha256": digest(completed.stderr), "stderr_size": len(completed.stderr)})
    if stdout_path is not None:
        stdout_path.parent.mkdir(parents=True, exist_ok=True)
        stdout_path.write_bytes(completed.stdout)
    if completed.returncode != 0:
        sys.stdout.buffer.write(completed.stdout)
        sys.stderr.buffer.write(completed.stderr)
        raise RuntimeError(f"validation command failed: {name}")
    return completed.stdout.decode("utf-8")


def artifact(root, path, media_type):
    content = (root / path).read_bytes()
    return {"path": path, "sha256": digest(content), "size": len(content), "media_type": media_type}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--clang", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path)
    args = parser.parse_args()
    source = args.source_dir.resolve()
    build = args.build_dir.resolve()
    clang = args.clang.resolve()
    root = args.artifact_dir.resolve() if args.artifact_dir else build / "whyvec-validation"
    for directory in (root / "checks", root / "compiler", root / "benchmark"):
        directory.mkdir(parents=True, exist_ok=True)

    packets = sorted((source / ".whyvec/agent-packets").glob("*.json"))
    if len(packets) != 1:
        raise RuntimeError(f"expected exactly one WhyVec agent packet, found {len(packets)}")
    packet = json.loads(packets[0].read_text(encoding="utf-8"))
    candidate = source / "src/kernel.c"
    outcomes = []
    commands = []
    run("repository_native_build", ["cmake", "--build", str(build)], outcomes, commands, cwd=source)
    run("repository_native_tests", ["ctest", "--test-dir", str(build), "--output-on-failure"], outcomes, commands, cwd=source)
    differential_text = run("fast_fallback_witness", [str(build / "whyvec_demo_differential")],
                            outcomes, commands, cwd=source, stdout_path=root / "checks/differential.json")
    differential_actual = json.loads(differential_text)

    sanitizer_binary = root / "checks/differential-sanitized"
    sanitizer_flags = [str(clang), "-std=c17", "-O1", "-g", "-fno-omit-frame-pointer",
                       "-fsanitize=address,undefined", "-I", str(source / "include"),
                       str(source / "tests/test_differential.c"), "-o", str(sanitizer_binary)]
    run("sanitizer_compile", sanitizer_flags, outcomes, commands, cwd=source)
    sanitizer_env = os.environ.copy()
    sanitizer_env["ASAN_OPTIONS"] = "detect_leaks=1:halt_on_error=1"
    sanitizer_env["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
    sanitizer_text = run("sanitizer_execute", [str(sanitizer_binary)], outcomes, commands,
                         cwd=source, stdout_path=root / "checks/sanitizer.json", env=sanitizer_env)
    sanitizer_actual = json.loads(sanitizer_text)
    if sanitizer_actual != differential_actual:
        raise RuntimeError("sanitizer path coverage differs from optimized differential coverage")

    production_sanitizer = root / "checks/production-sanitized"
    run("production_sanitizer_compile", [str(clang), "-std=c17", "-O1", "-g",
        "-fno-omit-frame-pointer", "-fsanitize=address,undefined", "-I", str(source / "include"),
        str(source / "tests/test_kernel.c"), str(candidate), str(source / "src/ffi.c"),
        "-o", str(production_sanitizer)], outcomes, commands, cwd=source)
    run("production_sanitizer_execute", [str(production_sanitizer)], outcomes, commands,
        cwd=source, env=sanitizer_env)

    yaml_path = root / "compiler/kernel.opt.yaml"
    run("production_optimization_compile", [str(clang), "-std=c17", "-O3", "-march=x86-64-v3",
        "-gline-tables-only", "-I", str(source / "include"), "-fsave-optimization-record",
        f"-foptimization-record-file={yaml_path}", "-c", str(candidate),
        "-o", str(root / "compiler/kernel.o")], outcomes, commands, cwd=source)
    records_path = root / "compiler/records.json"
    run("structured_optimization_assertions", [sys.executable, str(source / "tools/assert_opt_records.py"),
        "--source", str(candidate), "--yaml", str(yaml_path), "--output", str(records_path)],
        outcomes, commands, cwd=source)
    records = json.loads(records_path.read_text(encoding="utf-8"))

    csv_path = root / "benchmark/raw.csv"
    run("benchmark_execute", [str(build / "whyvec_demo_benchmark")], outcomes, commands,
        cwd=source, stdout_path=csv_path)
    raw_path = root / "benchmark/raw.json"
    summary_path = root / "benchmark/summary.json"
    run("benchmark_distribution_assertions", [sys.executable, str(source / "tools/analyze_benchmark.py"),
        "--csv", str(csv_path), "--raw", str(raw_path), "--summary", str(summary_path)],
        outcomes, commands, cwd=source)
    raw_benchmark = json.loads(raw_path.read_text(encoding="utf-8"))
    benchmark_summary = json.loads(summary_path.read_text(encoding="utf-8"))

    environment = {"kernel": platform.release(), "machine": platform.machine(),
                   "cpu_model": next((line.split(":", 1)[1].strip() for line in
                                      Path("/proc/cpuinfo").read_text().splitlines()
                                      if line.startswith("model name")), "unknown"),
                   "governor": "unknown", "affinity": sorted(os.sched_getaffinity(0)),
                   "python": platform.python_version()}
    (root / "environment.json").write_text(json.dumps(environment, indent=2, sort_keys=True) + "\n",
                                            encoding="utf-8")
    clang_version = subprocess.run([str(clang), "--version"], capture_output=True, text=True, check=True).stdout
    candidate_digest = digest(candidate.read_bytes())
    representative = benchmark_summary["representative"]
    report = {
        "schema_version": "1.2.0", "query_kind": "repair_validation",
        "evidence_strength": "validated_on_covered_executions",
        "obligation": {"analysis_id": packet["obligation"]["analysis_id"],
                       "semantic_digest": packet["obligation"]["semantic_digest"],
                       "family": "bound_object_disjoint_from_modified_region"},
        "toolchain": {"clang": {"invocation_path": str(args.clang), "resolved_path": str(clang),
                                  "binary_digest": digest(clang.read_bytes()), "version": clang_version},
                      "flags": ["-O3", "-march=x86-64-v3", "-std=c17"]},
        "sources": [{"path": str(path.relative_to(source)), "sha256": digest(path.read_bytes())} for path in
                    [candidate, source / "src/ffi.c", source / "include/whyvec_demo.h",
                     source / "tests/test_differential.c", source / "tests/benchmark.c"]],
        "candidate_source_sha256": candidate_digest, "commands": commands,
        "command_outcomes": outcomes,
        "validation_plan": {"schema_version": "1.0.0",
                            "required_checks": [{"id": outcome["name"], "property": outcome["name"]}
                                                for outcome in outcomes]},
        "differential": differential_actual,
        "sanitizer": {"clean": True, "covered": sanitizer_actual},
        "optimization": {"fast_path": "vectorized", "fallback": "missed",
                         "fast_path_line": records["fast_path_line"],
                         "fallback_line": records["fallback_line"], "records": records["records"]},
        "benchmark": {"raw": {"elements": representative["elements"],
                                "repetitions": raw_benchmark["sizes"][-1]["repetitions"],
                                "original_ns": representative["original_ns"],
                                "guarded_ns": representative["guarded_ns"]},
                      "summary": {"classification": benchmark_summary["classification"],
                                  "original_median_ns": representative["original_median_ns"],
                                  "original_mad_ns": representative["original_mad_ns"],
                                  "guarded_median_ns": representative["guarded_median_ns"],
                                  "guarded_mad_ns": representative["guarded_mad_ns"],
                                  "median_ratio": representative["median_ratio"],
                                  "decision_rule": benchmark_summary["decision_rule"]}},
        "environment": environment,
        "artifacts": [artifact(root, "checks/differential.json", "application/json"),
                      artifact(root, "checks/sanitizer.json", "application/json"),
                      artifact(root, "compiler/kernel.opt.yaml", "application/yaml"),
                      artifact(root, "compiler/records.json", "application/json"),
                      artifact(root, "benchmark/raw.csv", "text/csv"),
                      artifact(root, "benchmark/raw.json", "application/json"),
                      artifact(root, "benchmark/summary.json", "application/json"),
                      artifact(root, "environment.json", "application/json")],
        "artifact_path": str(root / "report.json"),
        "caveats": ["Covered executions do not establish full semantic equivalence.",
                    "The integer-address guard uses the recorded flat_uintptr_x86_64 policy.",
                    "External FFI callers remain uncertain and do not justify restrict."]}
    semantic = json.dumps(report, sort_keys=True, separators=(",", ":")).encode()
    report["analysis_id"] = "wv_" + digest(semantic)[:24]
    (root / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(root / "report.json"), "candidate_source_sha256": candidate_digest,
                      "benchmark": benchmark_summary["classification"], "commands": len(commands)}))


if __name__ == "__main__":
    main()
