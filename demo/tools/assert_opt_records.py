#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

import yaml


class OptimizationRecordLoader(yaml.SafeLoader):
    pass


def tagged_record(loader, suffix, node):
    record = loader.construct_mapping(node, deep=True)
    record["Kind"] = suffix
    return record


OptimizationRecordLoader.add_multi_constructor("!", tagged_record)


def source_line(lines, text):
    matches = [number for number, line in enumerate(lines, 1) if text in line]
    if len(matches) != 1:
        raise RuntimeError(f"expected one source line containing {text!r}, got {matches}")
    return matches[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--yaml", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    lines = args.source.read_text(encoding="utf-8").splitlines()
    fast_line = source_line(lines, "i < initial_count")
    fallback_line = source_line(lines, "i < *count")
    records = []
    for document in yaml.load_all(
        args.yaml.read_text(encoding="utf-8"), Loader=OptimizationRecordLoader
    ):
        if not isinstance(document, dict) or document.get("Pass") != "loop-vectorize":
            continue
        location = document.get("DebugLoc")
        if not isinstance(location, dict):
            continue
        arguments = document.get("Args") if isinstance(document.get("Args"), list) else []
        fields = {
            key: value
            for argument in arguments
            if isinstance(argument, dict)
            for key, value in argument.items()
        }
        name = document.get("Name")
        records.append({
            "pass": "loop-vectorize",
            "kind": document.get("Kind"),
            "name": name,
            "function": document.get("Function"),
            "line": int(location["Line"]),
            "column": int(location["Column"]),
            "outcome": "vectorized" if name == "Vectorized" else
                       "missed" if name in
                       {"UnsupportedUncountableLoop", "MissedDetails"} else "other",
            "vector_width": fields.get("VectorizationFactor") or fields.get("VectorizationWidth"),
            "interleave_count": fields.get("InterleaveCount") or fields.get("InterleavedCount"),
        })

    fast = [record for record in records if record["function"] == "add_vectors_" and
            record["line"] == fast_line and record["name"] == "Vectorized"]
    fallback_missed = [record for record in records if record["function"] == "add_vectors_" and
                       record["line"] == fallback_line and record["name"] == "MissedDetails"]
    fallback_reason = [record for record in records if record["function"] == "add_vectors_" and
                       record["line"] == fallback_line and
                       record["name"] == "UnsupportedUncountableLoop"]
    fallback_vectorized = [record for record in records if record["function"] == "add_vectors_" and
                           record["line"] == fallback_line and record["name"] == "Vectorized"]
    if len(fast) != 1 or len(fallback_missed) != 1 or len(fallback_reason) != 1 or fallback_vectorized:
        raise RuntimeError("exact fast/fallback loop-vectorize assertions failed")

    selected = [fast[0], fallback_reason[0], fallback_missed[0]]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"fast_path_line": fast_line,
                                       "fallback_line": fallback_line,
                                       "records": selected},
                                      indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(json.dumps({"fast_path": "vectorized", "fast_path_line": fast_line,
                      "fallback": "missed", "fallback_line": fallback_line}))


if __name__ == "__main__":
    main()
