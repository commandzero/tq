#!/usr/bin/env python3
"""Paired, correctness-gated probes using the frozen refactor measurement wrapper.

Requires elevated execution on macOS for /usr/bin/time -l peak RSS.
This narrows diagnosis to named workloads; it does not replace the full matrix.
"""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import statistics


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("report_directory", type=Path)
    parser.add_argument("cases", nargs="+")
    parser.add_argument("--runs", type=int, default=7)
    args = parser.parse_args()
    if platform.system() != "Darwin" or args.runs < 5:
        parser.error("requires macOS, elevated execution, and at least five pairs")
    spec = importlib.util.spec_from_file_location(
        "measurement", Path(__file__).with_name("native-format-regression.py"))
    measurement = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(measurement)
    root = args.root.resolve()
    manifest = json.loads((root / "manifest.json").read_text())
    baseline = json.loads((root / "baseline.json").read_text())
    measurement.validate_fixtures(root, manifest)
    metadata = measurement.metadata(root, baseline["runs"])
    measurement.validate_metadata(metadata, baseline)
    binaries = {"baseline": root / "tq-baseline", "candidate": args.candidate.resolve()}
    hashes = {name: measurement.digest(path.read_bytes()) for name, path in binaries.items()}
    if hashes["baseline"] != baseline["binary_sha256"]:
        raise RuntimeError("frozen baseline binary changed")
    cases = [case for case in manifest["cases"] if case["id"] in args.cases]
    if {case["id"] for case in cases} != set(args.cases):
        parser.error("unknown workload")
    directory = args.report_directory.resolve()
    directory.mkdir(parents=True)
    report = dict(metadata=metadata, binary_sha256=hashes, pairs=args.runs,
                  probe_sha256=measurement.digest(Path(__file__).read_bytes()),
                  method="alternating paired baseline/candidate", rows=[])
    for case in cases:
        env = os.environ.copy()
        env.pop("TQ_BENCH_FORCE_DOCUMENT", None)
        env.update(case["env"])
        suffix = [*case["args"], *[str(root / "inputs" / path) for path in case["files"]]]
        commands = {name: [str(path), *suffix] for name, path in binaries.items()}
        row = dict(id=case["id"], commands=commands, warmups={}, baseline=[], candidate=[])
        report["rows"].append(row)
        # A correctness-gated warmup uses the same wrapper and retains its RSS.
        for name in binaries:
            warmup = measurement.measure(commands[name], env, case["expected"], directory,
                                         f"{case['id']}-{name}-warmup")
            row["warmups"][name] = warmup
            (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            if warmup["status"] != "success":
                raise RuntimeError(f"{case['id']} {name}: warmup correctness failed")
        for index in range(args.runs):
            order = ["baseline", "candidate"] if index % 2 == 0 else ["candidate", "baseline"]
            for name in order:
                sample = measurement.measure(commands[name], env, case["expected"], directory,
                                             f"{case['id']}-{name}-{index}")
                row[name].append(sample)
                (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
                if sample["status"] != "success":
                    raise RuntimeError(f"{case['id']} {name}: measurement failed")
        row["time_percent"] = 100 * (statistics.median(s["wall_ns"] for s in row["candidate"]) /
                                     statistics.median(s["wall_ns"] for s in row["baseline"]) - 1)
        row["rss_percent"] = 100 * (max(s["rss_bytes"] for s in row["candidate"]) /
                                    max(s["rss_bytes"] for s in row["baseline"]) - 1)
        (directory / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{case['id']}: time {row['time_percent']:+.2f}%, RSS {row['rss_percent']:+.2f}%", flush=True)


if __name__ == "__main__":
    main()
