#!/usr/bin/env python3
"""Exercise the public explain-build CLI against an isolated Cargo repository."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


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
        rendered = " ".join(command)
        raise RuntimeError(
            f"command failed ({completed.returncode}): {rendered}\n"
            f"{completed.stdout}{completed.stderr}"
        )
    return completed


def write_base(repository: Path) -> None:
    source = repository / "src"
    source.mkdir(parents=True)
    (repository / "Cargo.toml").write_text(
        '[package]\nname = "causality-cli-fixture"\nversion = "0.1.0"\nedition = "2024"\n',
        encoding="utf-8",
    )
    host_marker = repository / "sandbox-escape-marker"
    private_tmp_marker = Path("/tmp") / f"whyvec-private-{repository.name}"
    (repository / "build.rs").write_text(
        "use std::net::{SocketAddr, TcpStream};\n"
        "use std::time::Duration;\n"
        "fn main() {\n"
        f"  if std::fs::write({json.dumps(str(host_marker))}, b\"escape\").is_ok() {{ panic!(\"host root was writable\"); }}\n"
        f"  std::fs::write({json.dumps(str(private_tmp_marker))}, b\"private\").unwrap();\n"
        "  let remote: SocketAddr = \"1.1.1.1:80\".parse().unwrap();\n"
        "  if TcpStream::connect_timeout(&remote, Duration::from_millis(100)).is_ok() { panic!(\"network was reachable\"); }\n"
        "}\n",
        encoding="utf-8",
    )
    (source / "api.rs").write_text(
        "pub fn measure(value: i32) -> usize {\n\n    value as usize\n}\n\n\n"
        "pub fn stable() -> usize { 1 }\n",
        encoding="utf-8",
    )
    (source / "consumer.rs").write_text(
        "use crate::api;\npub const HANDLER: fn(i32) -> usize = api::measure;\n",
        encoding="utf-8",
    )
    (source / "lib.rs").write_text(
        "pub mod api;\npub mod consumer;\npub mod other;\n"
        "pub fn run() -> usize { api::measure(7) }\n",
        encoding="utf-8",
    )
    (source / "other.rs").write_text(
        "pub fn label() -> &'static str { \"base\" }\n",
        encoding="utf-8",
    )


def initialize_git(repository: Path) -> None:
    run(["git", "init", "--quiet"], repository)
    run(["git", "config", "user.email", "whyvec@example.invalid"], repository)
    run(["git", "config", "user.name", "WhyVec Fixture"], repository)
    run(["git", "add", "."], repository)
    run(["git", "commit", "--quiet", "-m", "base"], repository)


def write_candidate(repository: Path) -> None:
    source = repository / "src"
    (source / "api.rs").write_text(
        "pub fn measure(value: &str) -> usize {\n\n    value.len()\n}\n\n\n"
        "pub fn stable() -> usize { 2 }\n",
        encoding="utf-8",
    )
    (source / "other.rs").write_text(
        "pub fn label() -> &'static str { \"changed\" }\n",
        encoding="utf-8",
    )
    (repository / "notes.txt").write_text("untracked context\n", encoding="utf-8")


def verify_report(report: dict[str, object], repository: Path) -> None:
    sandbox = report.get("toolchain", {}).get("sandbox", {})
    if sandbox.get("provider") != "bubblewrap":
        raise RuntimeError(f"build sandbox provenance is missing: {sandbox}")
    if not all(
        sandbox.get(field) is True
        for field in ("network_isolated", "host_root_read_only", "private_tmp")
    ):
        raise RuntimeError(f"build sandbox guarantees are incomplete: {sandbox}")
    if report.get("minimality") != "unique_minimal_in_declared_search":
        raise RuntimeError(f"unexpected minimality: {report.get('minimality')}")
    causal_sets = report.get("causal_sets")
    if not isinstance(causal_sets, list) or len(causal_sets) != 1:
        raise RuntimeError("expected exactly one causal set")
    causal_set = causal_sets[0]
    if not isinstance(causal_set, dict):
        raise RuntimeError("causal set is malformed")
    if causal_set.get("sufficient_files") != ["src/api.rs"]:
        raise RuntimeError(
            f"unexpected sufficient files: {causal_set.get('sufficient_files')}"
        )
    if causal_set.get("target_removed_from_full_patch") is not True:
        raise RuntimeError("removal witness did not suppress the target")
    suppressed = causal_set.get("diagnostics_suppressed_with_target")
    if not isinstance(suppressed, list) or len(suppressed) < 2:
        raise RuntimeError("expected the target and a co-suppressed diagnostic")
    refinements = report.get("hunk_refinements")
    if not isinstance(refinements, list) or len(refinements) != 1:
        raise RuntimeError("expected one hunk refinement")
    refinement = refinements[0]
    if not isinstance(refinement, dict) or len(refinement.get("hunks", [])) != 3:
        raise RuntimeError("expected three retained zero-context hunks")
    if refinement.get("grouping") != "rust_item":
        raise RuntimeError(f"Rust syntax grouping was not selected: {refinement}")
    groups = refinement.get("syntax_groups", [])
    if len(groups) != 2 or not any(
        group.get("symbol") == "measure" and len(group.get("member_hunks", [])) == 2
        for group in groups
    ):
        raise RuntimeError(f"separated edits in one function were not grouped: {groups}")
    hunk_sets = refinement.get("causal_sets")
    if not isinstance(hunk_sets, list) or len(hunk_sets) != 1:
        raise RuntimeError("expected one sufficient hunk set")
    if len(hunk_sets[0].get("sufficient_groups", [])) != 1:
        raise RuntimeError("expected one sufficient Rust item group")
    if len(hunk_sets[0].get("sufficient_hunks", [])) != 2:
        raise RuntimeError("expected both dependent function hunks in the sufficient group")
    if hunk_sets[0].get("target_removed_from_full_patch") is not True:
        raise RuntimeError("hunk removal witness did not suppress the target")
    artifact = report.get("artifact_path")
    if not isinstance(artifact, str) or not Path(artifact).is_file():
        raise RuntimeError("retained report was not written")

    schema = json.loads(
        (ROOT / "schemas/whyvec-build-report.schema.json").read_text(encoding="utf-8")
    )
    try:
        import jsonschema
    except ImportError as error:
        raise RuntimeError("jsonschema is required for report validation") from error
    jsonschema.Draft202012Validator(schema).validate(report)

    artifact_path = Path(artifact).resolve()
    if not artifact_path.is_relative_to(repository.resolve() / ".whyvec" / "analyses"):
        raise RuntimeError("report escaped the repository analysis directory")


def causal_projection(report: dict[str, object]) -> dict[str, object]:
    causal_sets = report["causal_sets"]
    assert isinstance(causal_sets, list)
    return {
        "target_id": report["target_diagnostic"]["id"],
        "atoms": report["atoms"],
        "evaluations": [
            {
                "subset": evaluation["subset"],
                "verdict": evaluation["verdict"],
                "unresolved_reason": evaluation["unresolved_reason"],
            }
            for evaluation in report["evaluations"]
        ],
        "minimality": report["minimality"],
        "stop_reason": report["stop_reason"],
        "causal_sets": [
            {
                "sufficient_atoms": causal_set["sufficient_atoms"],
                "sufficient_files": causal_set["sufficient_files"],
                "removal_subset": causal_set["removal_subset"],
                "target_removed_from_full_patch": causal_set[
                    "target_removed_from_full_patch"
                ],
                "suppressed_ids": [
                    diagnostic["id"]
                    for diagnostic in causal_set["diagnostics_suppressed_with_target"]
                ],
            }
            for causal_set in causal_sets
        ],
        "hunk_refinements": without_artifacts(report["hunk_refinements"]),
    }


def without_artifacts(value: object) -> object:
    if isinstance(value, dict):
        return {
            key: without_artifacts(child)
            for key, child in value.items()
            if key != "artifacts"
        }
    if isinstance(value, list):
        return [without_artifacts(child) for child in value]
    return value


def main() -> int:
    run(["cargo", "build", "--quiet", "-p", "whyvec-cli"], ROOT)
    binary = ROOT / "target" / "debug" / "whyvec"
    with tempfile.TemporaryDirectory(prefix="whyvec-build-cli-") as temporary:
        repository = Path(temporary)
        write_base(repository)
        initialize_git(repository)
        write_candidate(repository)
        ambiguous = subprocess.run(
            [
                str(binary),
                "explain-build",
                "--repository",
                str(repository),
                "--base",
                "HEAD",
                "--diagnostic",
                "E0308",
                "--",
                "cargo",
                "check",
            ],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if ambiguous.returncode == 0:
            raise RuntimeError("ambiguous diagnostic selection unexpectedly succeeded")
        if "matched 2 observations" not in ambiguous.stderr:
            raise RuntimeError(f"unexpected ambiguity refusal: {ambiguous.stderr}")
        if ambiguous.stderr.count("rustc:E0308:") < 2:
            raise RuntimeError("ambiguity refusal omitted stable diagnostic identities")

        completed = run(
            [
                str(binary),
                "explain-build",
                "--repository",
                str(repository),
                "--base",
                "HEAD",
                "--diagnostic",
                "E0308",
                "--at",
                "src/lib.rs",
                "--format",
                "json",
                "--",
                "cargo",
                "check",
            ],
            ROOT,
        )
        report = json.loads(completed.stdout)
        verify_report(report, repository)
        target = report["target_diagnostic"]
        if not isinstance(target, dict) or not isinstance(target.get("id"), str):
            raise RuntimeError("target diagnostic identity is missing")
        identity_completed = run(
            [
                str(binary),
                "explain-build",
                "--repository",
                str(repository),
                "--base",
                "HEAD",
                "--diagnostic",
                target["id"],
                "--format",
                "json",
                "--",
                "cargo",
                "check",
            ],
            ROOT,
        )
        identity_report = json.loads(identity_completed.stdout)
        verify_report(identity_report, repository)
        first_projection = causal_projection(report)
        second_projection = causal_projection(identity_report)
        if second_projection != first_projection:
            raise RuntimeError(
                "stable-identity rerun changed the causal result\n"
                f"first={json.dumps(first_projection, indent=2, sort_keys=True)}\n"
                f"second={json.dumps(second_projection, indent=2, sort_keys=True)}"
            )
        replayed = run(
            [str(binary), "replay-build", report["artifact_path"]],
            ROOT,
        )
        replay_result = json.loads(replayed.stdout)
        if replay_result.get("matched") is not True:
            raise RuntimeError(f"replay did not match: {replay_result}")
        if replay_result.get("semantic_digest") != report.get("semantic_digest"):
            raise RuntimeError("replay semantic digest differs from original")

        first_artifact = report["artifacts"][0]
        artifact_path = Path(report["artifact_path"]).parent / first_artifact["path"]
        artifact_path.chmod(0o600)
        with artifact_path.open("ab") as artifact_file:
            artifact_file.write(b"tamper")
        tampered = subprocess.run(
            [str(binary), "replay-build", report["artifact_path"]],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        if tampered.returncode == 0 or "artifact integrity check failed" not in tampered.stderr:
            raise RuntimeError("tampered retained artifact was not rejected")
        if (repository / "sandbox-escape-marker").exists():
            raise RuntimeError("sandboxed build script wrote into the source repository")
        if (Path("/tmp") / f"whyvec-private-{repository.name}").exists():
            raise RuntimeError("sandboxed build script's private /tmp write reached the host")
    print("build-causality CLI validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
