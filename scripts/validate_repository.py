#!/usr/bin/env python3
"""Validate repository contracts that do not require the compiler toolchain."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
IGNORED_DIRECTORIES = {".git", "target"}

REQUIRED_PATHS = (
    "README.md",
    "AGENTS.md",
    "PLAN.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "docs/PRODUCT_SPEC.md",
    "docs/ARCHITECTURE.md",
    "docs/SEMANTIC_MODEL.md",
    "docs/EXPERIMENT_PROTOCOL.md",
    "docs/REPORT_CONTRACT.md",
    "docs/AGENT_CONTRACT.md",
    "docs/TEST_STRATEGY.md",
    "docs/THREAT_MODEL.md",
    "docs/RISK_REGISTER.md",
    "schemas/whyvec-config.schema.json",
    "schemas/whyvec-report.schema.json",
    "schemas/fixture-manifest.schema.json",
    "fixtures/manifest.json",
    "toolchains/clang-21/profile.json",
    "integrations/codex/whyvec/.codex-plugin/plugin.json",
    "integrations/codex/whyvec/skills/whyvec-optimize/SKILL.md",
)

MARKDOWN_LINK = re.compile(r"(?<!!)\[[^]]*]\(([^)]+)\)")
IDENTIFIER = re.compile(r"^[a-z][a-z0-9-]+$")
DECLINE_CODE = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")


def load_json(path: Path, errors: list[str]) -> object | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        errors.append(f"{path.relative_to(ROOT)}: invalid JSON: {error}")
        return None


def validate_required_paths(errors: list[str]) -> None:
    for relative in REQUIRED_PATHS:
        if not (ROOT / relative).is_file():
            errors.append(f"missing required file: {relative}")


def validate_json_documents(errors: list[str]) -> None:
    for path in sorted(ROOT.rglob("*.json")):
        if not IGNORED_DIRECTORIES.intersection(path.parts):
            load_json(path, errors)


def validate_markdown_links(errors: list[str]) -> None:
    for document in sorted(ROOT.rglob("*.md")):
        if IGNORED_DIRECTORIES.intersection(document.parts):
            continue
        content = document.read_text(encoding="utf-8")
        for raw_target in MARKDOWN_LINK.findall(content):
            target = raw_target.strip().strip("<>")
            if target.startswith(("https://", "http://", "mailto:", "#")):
                continue
            target = unquote(target.split("#", 1)[0].split("?", 1)[0])
            if not target:
                continue
            resolved = (document.parent / target).resolve()
            try:
                resolved.relative_to(ROOT)
            except ValueError:
                errors.append(
                    f"{document.relative_to(ROOT)}: local link escapes repository: {raw_target}"
                )
                continue
            if not resolved.exists():
                errors.append(
                    f"{document.relative_to(ROOT)}: missing local link target: {raw_target}"
                )


def validate_fixture_manifest(errors: list[str]) -> None:
    path = ROOT / "fixtures/manifest.json"
    manifest = load_json(path, errors)
    if not isinstance(manifest, dict):
        return

    if manifest.get("$schema") != "../schemas/fixture-manifest.schema.json":
        errors.append("fixtures/manifest.json: unexpected schema reference")

    profile_id = manifest.get("toolchain_profile")
    profile = load_json(ROOT / "toolchains/clang-21/profile.json", errors)
    if isinstance(profile, dict) and profile.get("profile_id") != profile_id:
        errors.append("fixtures/manifest.json: toolchain profile id does not resolve")

    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        errors.append("fixtures/manifest.json: cases must be a non-empty array")
        return

    seen: set[str] = set()
    for index, case in enumerate(cases):
        prefix = f"fixtures/manifest.json: cases[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{prefix} must be an object")
            continue

        case_id = case.get("id")
        if not isinstance(case_id, str) or not IDENTIFIER.fullmatch(case_id):
            errors.append(f"{prefix}.id is not a stable identifier")
        elif case_id in seen:
            errors.append(f"{prefix}.id duplicates {case_id}")
        else:
            seen.add(case_id)

        source_value = case.get("source")
        if not isinstance(source_value, str):
            errors.append(f"{prefix}.source must be a string")
            continue
        source = (ROOT / "fixtures" / source_value).resolve()
        try:
            source.relative_to(ROOT / "fixtures")
        except ValueError:
            errors.append(f"{prefix}.source escapes fixtures directory")
            continue
        if not source.is_file():
            errors.append(f"{prefix}.source does not exist: {source_value}")
            continue

        selector = case.get("selector")
        if not isinstance(selector, dict):
            errors.append(f"{prefix}.selector must be an object")
            continue
        line = selector.get("line")
        if not isinstance(line, int) or line < 1:
            errors.append(f"{prefix}.selector.line must be a positive integer")
            continue
        lines = source.read_text(encoding="utf-8").splitlines()
        if line > len(lines):
            errors.append(f"{prefix}.selector.line is outside the source file")
        elif not re.search(r"\bfor\s*\(", lines[line - 1]):
            errors.append(f"{prefix}.selector.line does not select a for-loop")

        decline = case.get("expected_decline")
        if decline is not None and (
            not isinstance(decline, str) or not DECLINE_CODE.fullmatch(decline)
        ):
            errors.append(f"{prefix}.expected_decline is not a stable decline code")


def parse_frontmatter(path: Path, errors: list[str]) -> dict[str, str] | None:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "---":
        errors.append(f"{path.relative_to(ROOT)}: missing YAML frontmatter")
        return None
    try:
        end = lines.index("---", 1)
    except ValueError:
        errors.append(f"{path.relative_to(ROOT)}: unterminated YAML frontmatter")
        return None

    values: dict[str, str] = {}
    for line in lines[1:end]:
        if not line.strip():
            continue
        if ":" not in line:
            errors.append(f"{path.relative_to(ROOT)}: malformed frontmatter line: {line}")
            continue
        key, value = line.split(":", 1)
        values[key.strip()] = value.strip()
    return values


def validate_plugin(errors: list[str]) -> None:
    plugin_root = ROOT / "integrations/codex/whyvec"
    manifest_path = plugin_root / ".codex-plugin/plugin.json"
    manifest = load_json(manifest_path, errors)
    if not isinstance(manifest, dict):
        return

    if manifest.get("name") != "whyvec":
        errors.append(f"{manifest_path.relative_to(ROOT)}: plugin name must be whyvec")
    if manifest.get("skills") != "./skills/":
        errors.append(f"{manifest_path.relative_to(ROOT)}: unexpected skills path")

    interface = manifest.get("interface")
    if not isinstance(interface, dict) or interface.get("displayName") != "WhyVec":
        errors.append(f"{manifest_path.relative_to(ROOT)}: interface metadata is incomplete")

    skill = plugin_root / "skills/whyvec-optimize/SKILL.md"
    frontmatter = parse_frontmatter(skill, errors)
    if frontmatter is None:
        return
    if frontmatter.get("name") != "whyvec-optimize":
        errors.append(f"{skill.relative_to(ROOT)}: skill name does not match directory")
    description = frontmatter.get("description", "")
    if len(description) < 80 or "WhyVec" not in description:
        errors.append(f"{skill.relative_to(ROOT)}: description lacks trigger detail")


def validate_text_files(errors: list[str]) -> None:
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or IGNORED_DIRECTORIES.intersection(path.parts):
            continue
        if path.suffix not in {".md", ".json", ".toml", ".py", ".yml", ".yaml", ".c"}:
            continue
        raw = path.read_bytes()
        if raw and not raw.endswith(b"\n"):
            errors.append(f"{path.relative_to(ROOT)}: text file must end with a newline")
        if b"[T" + b"ODO" in raw:
            errors.append(f"{path.relative_to(ROOT)}: unresolved template marker")


def main() -> int:
    errors: list[str] = []
    validate_required_paths(errors)
    validate_json_documents(errors)
    validate_markdown_links(errors)
    validate_fixture_manifest(errors)
    validate_plugin(errors)
    validate_text_files(errors)

    if errors:
        print("repository validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print("repository validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
