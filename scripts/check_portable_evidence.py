#!/usr/bin/env python3
"""Reject machine-private paths and raw reasoning telemetry in shareable evidence."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PORTABLE = [ROOT / "evidence/codex-live", ROOT / "evidence/real-world"]
FORBIDDEN = [
    re.compile(r"/(?:home|Users|tmp)/[^<\s\"']"),
    re.compile(r"[A-Za-z]:\\Users\\"),
    re.compile(r'"type"\s*:\s*"reasoning"'),
    re.compile(r'"(?:input_tokens|output_tokens|reasoning_output_tokens)"'),
]


def main() -> int:
    errors = []
    for root in PORTABLE:
        for path in sorted(item for item in root.rglob("*") if item.is_file()):
            text = path.read_text(encoding="utf-8", errors="replace")
            for pattern in FORBIDDEN:
                if pattern.search(text):
                    errors.append(f"{path.relative_to(ROOT)}: matched {pattern.pattern}")
    if errors:
        raise SystemExit("portable evidence validation failed:\n  - " + "\n  - ".join(errors))
    print("portable evidence validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
