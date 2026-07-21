#!/usr/bin/env python3
import argparse
import csv
import json
import statistics
from pathlib import Path


def median_absolute_deviation(values):
    center = statistics.median(values)
    return statistics.median(abs(value - center) for value in values)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--csv", type=Path, required=True)
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--summary", type=Path, required=True)
    args = parser.parse_args()

    distributions = {}
    with args.csv.open(newline="", encoding="utf-8") as source:
        for row in csv.DictReader(source):
            size = int(row["size"])
            bucket = distributions.setdefault(size, {"original_ns": [], "guarded_ns": [],
                                                       "orders": [], "repetitions": int(row["repetitions"])})
            bucket["original_ns"].append(int(row["original_ns"]))
            bucket["guarded_ns"].append(int(row["guarded_ns"]))
            bucket["orders"].append(row["order"])

    if sorted(distributions) != [8, 31, 64, 257, 1024, 4096, 16384, 65536]:
        raise RuntimeError("benchmark size distribution changed")
    per_size = []
    for size, bucket in sorted(distributions.items()):
        if len(bucket["original_ns"]) != 31 or len(bucket["guarded_ns"]) != 31:
            raise RuntimeError(f"size {size} does not have 31 raw samples")
        original_median = statistics.median(bucket["original_ns"])
        guarded_median = statistics.median(bucket["guarded_ns"])
        original_mad = median_absolute_deviation(bucket["original_ns"])
        guarded_mad = median_absolute_deviation(bucket["guarded_ns"])
        separated = original_median - guarded_median > 3 * (original_mad + guarded_mad)
        per_size.append({"elements": size, "original_median_ns": original_median,
                         "guarded_median_ns": guarded_median,
                         "original_mad_ns": original_mad, "guarded_mad_ns": guarded_mad,
                         "median_ratio": original_median / guarded_median,
                         "separated": separated})

    material = [entry for entry in per_size if entry["elements"] >= 257]
    median_ratio = statistics.median(entry["median_ratio"] for entry in material)
    separated_sizes = sum(entry["separated"] for entry in material)
    classification = "measured_improvement" if median_ratio > 1.10 and separated_sizes >= 4 else "noise_decline"
    largest = distributions[max(distributions)]
    raw = {"seed": "0x32c56bc2", "warmup_rounds": 7,
           "sizes": [{"elements": size, **bucket} for size, bucket in sorted(distributions.items())]}
    summary = {"classification": classification,
               "decision_rule": "for sizes >=257, median speedup >1.10 and at least 4/5 median separations exceed three times summed MAD",
               "material_size_median_ratio": median_ratio,
               "material_size_separated_count": separated_sizes,
               "per_size": per_size,
               "representative": {"elements": max(distributions),
                                  "repetitions": largest["repetitions"],
                                  "original_median_ns": statistics.median(largest["original_ns"]),
                                  "original_mad_ns": median_absolute_deviation(largest["original_ns"]),
                                  "guarded_median_ns": statistics.median(largest["guarded_ns"]),
                                  "guarded_mad_ns": median_absolute_deviation(largest["guarded_ns"]),
                                  "median_ratio": statistics.median(largest["original_ns"]) /
                                                  statistics.median(largest["guarded_ns"]),
                                  "original_ns": largest["original_ns"],
                                  "guarded_ns": largest["guarded_ns"]}}
    args.raw.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    args.summary.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"classification": classification, "material_size_median_ratio": median_ratio,
                      "material_size_separated_count": separated_sizes}))
    if classification != "measured_improvement":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
