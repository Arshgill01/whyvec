#!/usr/bin/env python3
"""Retain observable Codex events without hidden reasoning or machine-private paths."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ALLOWED_ITEMS = {"agent_message", "command_execution", "file_change"}


def scrub(value: object, replacements: list[tuple[str, str]]) -> object:
    if isinstance(value, str):
        for source, replacement in replacements:
            value = value.replace(source, replacement)
        return value
    if isinstance(value, list):
        return [scrub(item, replacements) for item in value]
    if isinstance(value, dict):
        return {key: scrub(item, replacements) for key, item in value.items()}
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--product-root", type=Path, required=True)
    arguments = parser.parse_args()

    replacements = [
        (str(arguments.workspace.resolve()), "<demo-repository>"),
        (str(arguments.product_root.resolve()), "<whyvec-repository>"),
        (str(Path.home()), "<home>"),
        (Path.home().name, "<user>"),
    ]
    retained: list[dict[str, object]] = []
    for line_number, line in enumerate(
        arguments.input.read_text(encoding="utf-8", errors="replace").splitlines(), 1
    ):
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        event_type = event.get("type")
        if event_type == "thread.started":
            retained.append({"type": event_type, "thread_id": event.get("thread_id")})
            continue
        if event_type == "turn.completed":
            # Usage and reasoning-token telemetry are intentionally not retained.
            retained.append({"type": event_type})
            continue
        if event_type != "item.completed":
            continue
        item = event.get("item")
        if not isinstance(item, dict) or item.get("type") not in ALLOWED_ITEMS:
            continue
        if "codex-session.raw" in str(item.get("aggregated_output", "")):
            item = dict(item)
            item["aggregated_output"] = (
                "<redacted: command output included the raw Codex event stream>"
            )
        retained.append(
            {
                "type": event_type,
                "source_line": line_number,
                "item": scrub(item, replacements),
            }
        )

    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("x", encoding="utf-8") as destination:
        for event in retained:
            destination.write(json.dumps(event, sort_keys=True) + "\n")
    print(json.dumps({"events": len(retained), "output": str(arguments.output)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
