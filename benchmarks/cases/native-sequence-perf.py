#!/usr/bin/env python3
"""Paired sequence extraction checks. Run elevated on macOS for time -l RSS.

Keep ROOT in the separate tq-benchmarks checkout. Baseline must be a frozen
binary. Every sample checks output against independently constructed row IDs.
"""

import argparse
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--rows", type=int, default=131072)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--require-improvement", type=float, default=0.10)
    args = parser.parse_args()
    if platform.system() != "Darwin" or args.rows < 1 or args.runs < 3:
        parser.error("requires macOS, positive row count, and at least three pairs")
    if not math.isfinite(args.require_improvement) or not 0 <= args.require_improvement < 1:
        parser.error("improvement must be a finite fraction in [0, 1)")
    spec = importlib.util.spec_from_file_location(
        "measurement", Path(__file__).with_name("native-format-regression.py"))
    measurement = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(measurement)
    root = args.root.resolve()
    root.mkdir(parents=True)  # Refuse to overwrite any prior campaign.
    binaries = {name: str(path.resolve()) for name, path in
                [("baseline", args.baseline), ("candidate", args.candidate)]}
    env = os.environ.copy()
    env.pop("TQ_BENCH_FORCE_DOCUMENT", None)
    rows = [{"id": i, "active": i % 2 == 0, "label": f"native-row-{i}"}
            for i in range(args.rows)]
    lines = [json.dumps(row, separators=(",", ":")) + "\n" for row in rows]
    fixtures = {
        "jsonl": "".join(lines),
        "json-seq": "".join("\x1e" + line for line in lines),
        "toon-seq": "".join(
            f"\x1eid: {r['id']}\nactive: {str(r['active']).lower()}\nlabel: {r['label']}\n"
            for r in rows),
    }
    for fmt, separator in [("csv", ","), ("tsv", "\t")]:
        fixtures[fmt] = separator.join(["id", "active", "label"]) + "\n" + "".join(
            separator.join([str(r["id"]), str(r["active"]).lower(), r["label"]]) + "\n"
            for r in rows)
    for fmt, data in fixtures.items():
        (root / f"input.{fmt}").write_text(data)
    report = dict(host=platform.platform(), rows=args.rows, runs=args.runs, warmups=1,
                  thresholds=dict(time_ratio=1 - args.require_improvement,
                                  json_sequence_rss_ratio=1 - args.require_improvement,
                                  other_rss_ratio=1.1),
                  toolchain=measurement.command(["rustc", "-Vv"]).decode(),
                  script_sha256=measurement.digest(Path(__file__).read_bytes()),
                  measurement_sha256=measurement.digest(Path(spec.origin).read_bytes()),
                  binaries={k: dict(path=v, sha256=measurement.digest(Path(v).read_bytes()))
                            for k, v in binaries.items()},
                  fixtures={fmt: measurement.digest(data.encode())
                            for fmt, data in fixtures.items()}, cases=[])
    passed = True
    for fmt in ["csv", "tsv", "json-seq", "toon-seq", "jsonl"]:
        output_format = "json-seq" if fmt == "json-seq" else "json"
        controls = ["-r"] if fmt in ["csv", "tsv"] else ["-c"]
        suffix = ["-i", fmt, "-o", output_format, *controls, ".id", str(root / f"input.{fmt}")]
        expected = measurement.digest("".join(
            ("\x1e" if fmt == "json-seq" else "") + f"{i}\n"
            for i in range(args.rows)).encode())
        case = dict(format=fmt, args=suffix, expected_sha256=expected,
                    validation={}, warmups={}, baseline=[], candidate=[])
        report["cases"].append(case)
        # Verify the entire ordered Document mapping before admitting fixtures.
        for label, binary in binaries.items():
            decoded = measurement.observed(
                [binary, "-i", fmt, "-o", "json", "-c", ".", str(root / f"input.{fmt}")],
                env=env, stdout=subprocess.PIPE)
            (root / f"{fmt}-{label}-validation.stderr").write_bytes(decoded["stderr"])
            validation = dict(status=decoded["status"], returncode=decoded["returncode"])
            if validation["status"] == "success" and decoded["stdout"] != fixtures["jsonl"].encode():
                validation["status"] = "incorrect"
            case["validation"][label] = validation
            (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            if validation["status"] != "success":
                (root / f"{fmt}-{label}-validation.failed-output").write_bytes(decoded["stdout"])
                raise RuntimeError(f"{label} {fmt}: fixture validation failed")
            warmup = measurement.measure([binary, *suffix], env, expected, root, f"{fmt}-{label}-warmup")
            case["warmups"][label] = warmup
            (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            if warmup["status"] != "success":
                raise RuntimeError(f"{label} {fmt}: warmup failed")
        for index in range(args.runs):
            order = ["baseline", "candidate"] if index % 2 == 0 else ["candidate", "baseline"]
            for label in order:
                sample = measurement.measure([binaries[label], *suffix], env, expected, root,
                                             f"{fmt}-{index}-{label}")
                case[label].append(sample)
                (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
                if sample["status"] != "success":
                    raise RuntimeError(f"{label} {fmt}: measurement failed")
        for label in binaries:
            case[label + "_median_ms"] = statistics.median(s["wall_ns"] for s in case[label]) / 1e6
            case[label + "_rss_bytes"] = max(s["rss_bytes"] for s in case[label])
        case["time_ratio"] = case["candidate_median_ms"] / case["baseline_median_ms"]
        case["rss_ratio"] = case["candidate_rss_bytes"] / case["baseline_rss_bytes"]
        case["passed"] = (case["time_ratio"] <= 1 - args.require_improvement
                          and case["rss_ratio"] <= 1.1
                          and (fmt != "json-seq" or case["rss_ratio"] <= 1 - args.require_improvement))
        passed &= case["passed"]
        (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{fmt}: {case['baseline_median_ms']:.2f} -> {case['candidate_median_ms']:.2f} ms; "
              f"RSS {case['baseline_rss_bytes']} -> {case['candidate_rss_bytes']}; "
              f"{'PASS' if case['passed'] else 'FAIL'}", flush=True)
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
