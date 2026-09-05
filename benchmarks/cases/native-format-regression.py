#!/usr/bin/env python3
"""Frozen, correctness-gated native-format refactor measurements on macOS.

Run elevated outside the sandbox. Generated fixtures and samples belong in the
separate tq-benchmarks checkout. Candidate runs reuse the baseline's fixtures.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import statistics
import subprocess
import time


def digest(data):
    return hashlib.sha256(data).hexdigest()


def metadata(root, runs):
    return dict(measurement_script_sha256=digest(Path(__file__).read_bytes()),
                host=platform.platform(), machine=platform.machine(),
                toolchain=command(["rustc", "-Vv"]).decode(),
                build="cargo build --release -p tq-cli -p tq-test-support --bins",
                manifest_sha256=digest((root / "manifest.json").read_bytes()),
                runs=runs, warmups=1)


def validate_fixtures(root, manifest):
    for name, expected in manifest["fixtures"].items():
        if digest((root / "inputs" / name).read_bytes()) != expected:
            raise RuntimeError(f"fixture changed: {name}")


def validate_metadata(expected, actual):
    for key, value in expected.items():
        if actual.get(key) != value:
            raise RuntimeError(f"incomparable measurement: {key}")


def command(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, timeout=300, **kwargs).stdout


def observed(args, **kwargs):
    # time is a wrapper process. Kill its process group on timeout so the
    # measured executable cannot survive and contaminate subsequent samples.
    with subprocess.Popen(args, stderr=subprocess.PIPE, start_new_session=True, **kwargs) as process:
        try:
            stdout, stderr = process.communicate(timeout=300)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            return dict(status="timeout", returncode=process.returncode, stdout=stdout or b"", stderr=stderr or b"")
        returncode = process.returncode
    stderr = stderr or b""
    status = "success"
    if returncode < 0:
        status = "signal"
    elif returncode:
        status = "resource-limit" if b"resource limit" in stderr.lower() else "unsupported" if b"unsupported" in stderr.lower() else "incorrect"
    return dict(status=status, returncode=returncode, stdout=stdout or b"", stderr=stderr)


def measure(invocation, env, expected, directory, label):
    with (directory / "output").open("wb") as output:
        start = time.perf_counter_ns()
        result = observed(["/usr/bin/time", "-l", *invocation], env=env, stdout=output)
        elapsed = time.perf_counter_ns() - start
    (directory / f"{label}.time").write_bytes(result["stderr"])
    if result["status"] != "success":
        shutil.copy2(directory / "output", directory / f"{label}.failed-output")
        return dict(status=result["status"], returncode=result["returncode"], wall_ns=elapsed)
    rss = re.search(rb"(\d+)\s+maximum resident set size", result["stderr"])
    if rss is None or int(rss[1]) == 0:
        raise RuntimeError("invalid RSS: discard campaign and rerun elevated")
    if digest((directory / "output").read_bytes()) != expected:
        shutil.copy2(directory / "output", directory / f"{label}.incorrect-output")
        return dict(status="incorrect", returncode=0, wall_ns=elapsed, rss_bytes=int(rss[1]))
    return dict(status="success", wall_ns=elapsed, rss_bytes=int(rss[1]))


def investigate(root, binary, runs):
    manifest = json.loads((root / "manifest.json").read_text())
    candidate = json.loads((root / "candidate.json").read_text())
    baseline = json.loads((root / "baseline.json").read_text())
    validate_fixtures(root, manifest)
    checked = metadata(root, runs)
    validate_metadata(checked, baseline)
    validate_metadata(checked, candidate)
    if digest((root / "tq-baseline").read_bytes()) != baseline["binary_sha256"]:
        raise RuntimeError("baseline binary changed before investigation")
    affected = {row["id"] for row in candidate["rows"]
                if any(row.get(metric + "_comparison", {}).get("percent", 0) >= 10
                       for metric in ["median_wall_ns", "maximum_rss_bytes"])}
    directory = root / "investigation"
    directory.mkdir()
    report = dict(method="alternating baseline/candidate order, three rounds of paired samples",
                  baseline_sha256=digest((root / "tq-baseline").read_bytes()),
                  candidate_sha256=digest(Path(binary).read_bytes()), metadata=checked, rows=[])
    if report["candidate_sha256"] != candidate["binary_sha256"]:
        raise RuntimeError("candidate binary changed before investigation")
    for case in manifest["cases"]:
        if case["id"] not in affected:
            continue
        env = os.environ.copy()
        env.pop("TQ_BENCH_FORCE_DOCUMENT", None)
        env.update(case["env"])
        suffix = [*case["args"], *[str(root / "inputs" / p) for p in case["files"]]]
        binaries = {"baseline": str(root / "tq-baseline"), "candidate": binary}
        row = dict(id=case["id"], baseline=[], candidate=[])
        for label, executable in binaries.items():
            gate = observed([executable, *suffix], env=env, stdout=subprocess.PIPE)
            (directory / f"{case['id']}-{label}.gate-stderr").write_bytes(gate["stderr"])
            if gate["status"] != "success" or digest(gate["stdout"]) != case["expected"]:
                (directory / f"{case['id']}-{label}.failed-output").write_bytes(gate["stdout"])
                row["status"] = gate["status"] if gate["status"] != "success" else "incorrect"
                row["failed_binary"] = label
                row["returncode"] = gate["returncode"]
                report["rows"].append(row)
                (root / "investigation.json").write_text(json.dumps(report, indent=2) + "\n")
                raise RuntimeError("paired correctness gate failed; evidence preserved")
        for index in range(runs * 3):
            order = ["baseline", "candidate"] if index % 2 == 0 else ["candidate", "baseline"]
            for label in order:
                sample = measure([binaries[label], *suffix], env, case["expected"], directory,
                                 f"{case['id']}-{index}-{label}")
                row[label].append(sample)
                if sample["status"] != "success":
                    report["rows"].append(row)
                    (root / "investigation.json").write_text(json.dumps(report, indent=2) + "\n")
                    raise RuntimeError("paired measurement failed; evidence preserved")
        row["time_percent"] = 100 * (statistics.median(s["wall_ns"] for s in row["candidate"]) /
                                     statistics.median(s["wall_ns"] for s in row["baseline"]) - 1)
        row["rss_percent"] = 100 * (max(s["rss_bytes"] for s in row["candidate"]) /
                                    max(s["rss_bytes"] for s in row["baseline"]) - 1)
        report["rows"].append(row)
        print(f"{case['id']}: paired time {row['time_percent']:+.2f}%, RSS {row['rss_percent']:+.2f}%", flush=True)
    (root / "investigation.json").write_text(json.dumps(report, indent=2) + "\n")


def prepare(root, binary, reference):
    inputs = root / "inputs"
    inputs.mkdir()
    cases = []
    for label, count in [("small", 8), ("large", 131072)]:
        rows = [{"id": i, "name": f"item-{i}", "active": i % 2 == 0}
                for i in range(count)]
        array = (json.dumps(rows, separators=(",", ":")) + "\n").encode()
        lines = b"".join((json.dumps(row, separators=(",", ":")) + "\n").encode()
                         for row in rows)
        for fmt, data in [("json", array), ("json5", array), ("jsonl", lines)]:
            (inputs / f"{label}.{fmt}").write_bytes(data)
        for fmt in ["yaml", "toon"]:
            framing = ["--unframed"] if fmt == "toon" else []
            data = command([binary, "-i", "json", "-o", fmt, *framing, "."], input=array)
            decoded = command([binary, "-i", fmt, "-o", "json", "-c", "."], input=data)
            if decoded != array:
                raise RuntimeError(f"full ordered fixture validation failed: {label}.{fmt}")
            (inputs / f"{label}.{fmt}").write_bytes(data)
        data = command([binary, "-i", "jsonl", "-o", "toon", "."], input=lines)
        if command([binary, "-i", "toon-seq", "-o", "json", "-c", "."], input=data) != lines:
            raise RuntimeError(f"full ordered fixture validation failed: {label}.toon-seq")
        (inputs / f"{label}.toon-seq").write_bytes(data)

        for fmt in ["json", "json5", "yaml", "toon", "jsonl", "toon-seq"]:
            sequence = fmt in ["jsonl", "toon-seq"]
            query = ".id" if sequence else ".[0].id"
            expected = command([reference, "-c", query], input=lines if sequence else array)
            cases.append(dict(id=f"{label}-{fmt}-extract", args=["-i", fmt, "-o", "json", "-c", query],
                              files=[f"{label}.{fmt}"], expected=digest(expected), env={}))
        for mode, extra, env in [("automatic", [], {}),
                                 ("document", [], {"TQ_BENCH_FORCE_DOCUMENT": "1"}),
                                 ("events", ["--stream"], {})]:
            expected = command([reference, "-c", *extra, "."], input=array)
            cases.append(dict(id=f"{label}-json-{mode}", args=["-i", "json", "-o", "json", "-c", *extra, "."],
                              files=[f"{label}.json"], expected=digest(expected), env=env))
        for fmt in ["jsonl", "toon-seq"]:
            expected = command([reference, "-c", ".id, (inputs | .id)"], input=lines + lines)
            cases.append(dict(id=f"{label}-{fmt}-remaining-multisource", args=["-i", fmt, "-o", "json", "-c", ".id, (inputs | .id)"],
                              files=[f"{label}.{fmt}"] * 2, expected=digest(expected), env={}))
        for fmt in ["json", "jsonl", "yaml", "toon-seq"]:
            # Independently check generated native output by decoding to the
            # known ordered JSON rows before admitting its byte digest.
            args = ["-i", "jsonl", "-o", "toon" if fmt == "toon-seq" else fmt, "."]
            paths = [str(inputs / f"{label}.jsonl")] * 2
            encoded = command([binary, *args, *paths])
            decoded = command([binary, "-i", fmt, "-o", "json", "-c", "."], input=encoded)
            if decoded != lines + lines:
                raise RuntimeError(f"output correctness failed: {label}-{fmt}")
            cases.append(dict(id=f"{label}-{fmt}-output-multisource", args=args,
                              files=[f"{label}.jsonl"] * 2, expected=digest(encoded), env={}))
    manifest = dict(cases=cases, fixtures={p.name: digest(p.read_bytes()) for p in inputs.iterdir()},
                    reference_version=command([reference, "--version"]).decode().strip(),
                    reference_sha256=digest(Path(reference).read_bytes()))
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["baseline", "candidate", "investigate"])
    parser.add_argument("root", type=Path)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--reference", type=Path)
    parser.add_argument("--runs", type=int, default=7)
    args = parser.parse_args()
    if platform.system() != "Darwin" or args.runs < 5:
        parser.error("requires macOS, elevated execution, and at least five samples")
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    binary = str(args.binary.resolve())
    if args.phase == "investigate":
        investigate(root, binary, args.runs)
        return
    if args.phase == "baseline":
        if (root / "manifest.json").exists() or (root / "inputs").exists():
            parser.error("baseline exists; choose a fresh archive directory")
        if args.reference is None:
            parser.error("baseline requires --reference with jq 1.8.x")
        try:
            prepare(root, binary, str(args.reference.resolve()))
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            (root / "preparation-failure.stdout").write_bytes(error.stdout or b"")
            (root / "preparation-failure.stderr").write_bytes(error.stderr or b"")
            (root / "preparation-failure.txt").write_text(str(error) + "\n")
            raise
        shutil.copy2(binary, root / "tq-baseline")
    manifest = json.loads((root / "manifest.json").read_text())
    validate_fixtures(root, manifest)
    checked = metadata(root, args.runs)
    if args.phase == "candidate":
        validate_metadata(checked, json.loads((root / "baseline.json").read_text()))
    report = dict(**checked, revision=command(["git", "rev-parse", "HEAD"]).decode().strip(),
                  binary_sha256=digest(Path(binary).read_bytes()),
                  rows=[])
    samples = root / args.phase
    samples.mkdir()
    for case in manifest["cases"]:
        invocation = [binary, *case["args"], *[str(root / "inputs" / p) for p in case["files"]]]
        env = os.environ.copy()
        env.pop("TQ_BENCH_FORCE_DOCUMENT", None)
        env.update(case["env"])
        gate = observed(invocation, env=env, stdout=subprocess.PIPE)
        (samples / f"{case['id']}.gate-stderr").write_bytes(gate["stderr"])
        if gate["status"] != "success" or digest(gate["stdout"]) != case["expected"]:
            (samples / f"{case['id']}.incorrect-output").write_bytes(gate["stdout"])
            report["rows"].append(dict(id=case["id"], command=invocation,
                                       status=gate["status"] if gate["status"] != "success" else "incorrect", returncode=gate["returncode"], samples=[]))
            (root / f"{args.phase}.json").write_text(json.dumps(report, indent=2) + "\n")
            print(f"{case['id']}: incorrect, not timed", flush=True)
            continue
        row = dict(id=case["id"], command=invocation, env=case["env"], status="success", samples=[])
        for index in range(args.runs):
            row["samples"].append(measure(invocation, env, case["expected"], samples,
                                          f"{case['id']}-{index}"))
            if row["samples"][-1]["status"] != "success":
                row["status"] = row["samples"][-1]["status"]
                break
        if row["status"] != "success":
            report["rows"].append(row)
            (root / f"{args.phase}.json").write_text(json.dumps(report, indent=2) + "\n")
            print(f"{case['id']}: {row['status']}, not comparable", flush=True)
            continue
        row["median_wall_ns"] = statistics.median(s["wall_ns"] for s in row["samples"])
        row["maximum_rss_bytes"] = max(s["rss_bytes"] for s in row["samples"])
        report["rows"].append(row)
        (root / f"{args.phase}.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{case['id']}: {row['median_wall_ns']/1e6:.2f} ms, {row['maximum_rss_bytes']} RSS bytes", flush=True)
    if args.phase == "candidate":
        baseline = json.loads((root / "baseline.json").read_text())
        validate_metadata(checked, baseline)
        for old, new in zip(baseline["rows"], report["rows"], strict=True):
            if old["id"] != new["id"]:
                raise RuntimeError("workload mismatch")
            if len(old["samples"]) != args.runs or len(new["samples"]) != args.runs or old.get("status", "success") != "success" or new.get("status", "success") != "success":
                new["comparison"] = "not-comparable: correctness gate failed"
                continue
            for metric in ["median_wall_ns", "maximum_rss_bytes"]:
                change = 100 * (new[metric] / old[metric] - 1)
                status = "investigate" if change > 25 else "missed" if change >= 10 else "met"
                new[metric + "_comparison"] = dict(percent=change, status=status)
        (root / "candidate.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
