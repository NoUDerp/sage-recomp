#!/usr/bin/env python3
"""Mint golden vectors for sage-port tasks from the openbfme_core C++ reference.

Owner-only. Contributors and verifier guests never run this: they consume the
committed `goldens/<task-id>.json` files, which hold eight case-group digests and
eight visible example rows per task.

    tools/mint_goldens.py --all              # build reference, mint every task
    tools/mint_goldens.py --task port-0003   # mint one task
    tools/mint_goldens.py --all --check      # fail if a committed golden drifted

Heavy artefacts (upstream checkout, CMake build, ccache) live under `.cache/`,
which is gitignored and reused between sessions.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT / "plugins" / "sage-port"))

import runner  # noqa: E402  (path set above)

CACHE = PROJECT_ROOT / ".cache" / "reference"
SRC = CACHE / "src"
BUILD = CACHE / "build"
BIN = CACHE / "harness"
STAMP = CACHE / "build.stamp"
UPSTREAM = PROJECT_ROOT / "reference" / "upstream.toml"
HARNESS_DIR = PROJECT_ROOT / "reference" / "harness"
GOLDEN_DIR = PROJECT_ROOT / "goldens"


def log(message: str) -> None:
    print(f"[mint] {message}", flush=True)


def upstream_config() -> dict:
    return tomllib.loads(UPSTREAM.read_text(encoding="utf-8"))["upstream"]


def _run(argv: list[str], **kwargs) -> subprocess.CompletedProcess:
    result = subprocess.run(argv, text=True, **kwargs)
    if result.returncode != 0:
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(argv)}")
    return result


def ensure_checkout(config: dict) -> None:
    commit = config["commit"]
    if (SRC / ".git").exists():
        head = subprocess.run(["git", "-C", str(SRC), "rev-parse", "HEAD"],
                              capture_output=True, text=True).stdout.strip()
        if head == commit:
            return
        log(f"fetching upstream {commit[:12]}")
        _run(["git", "-C", str(SRC), "fetch", "--depth", "1", "origin", commit])
        _run(["git", "-C", str(SRC), "checkout", "--force", commit])
        return
    SRC.parent.mkdir(parents=True, exist_ok=True)
    log(f"cloning {config['repo']} at {commit[:12]} (blobless, no LFS)")
    env = dict(os.environ, GIT_LFS_SKIP_SMUDGE="1")
    _run(["git", "clone", "--filter=blob:none", "--no-checkout", config["repo"], str(SRC)],
         env=env)
    _run(["git", "-C", str(SRC), "fetch", "--depth", "1", "origin", commit], env=env)
    _run(["git", "-C", str(SRC), "checkout", "--force", commit], env=env)


def ensure_build(config: dict) -> None:
    commit = config["commit"]
    flags = list(config.get("cmake_flags", []))
    stamp = json.dumps({"commit": commit, "flags": flags}, sort_keys=True)
    if STAMP.exists() and STAMP.read_text(encoding="utf-8") == stamp \
            and (BUILD / "libopenbfme_core.a").exists():
        log("reference build is current (cached)")
        return
    for tool in ("cmake", "ninja", "g++"):
        if shutil.which(tool) is None:
            raise SystemExit(f"{tool} is required to mint goldens; run scripts/coordinator.sh setup")
    launcher = []
    if shutil.which("ccache"):
        launcher = ["-DCMAKE_CXX_COMPILER_LAUNCHER=ccache"]
    env = dict(os.environ)
    env.setdefault("CCACHE_DIR", str(PROJECT_ROOT / ".cache" / "ccache"))
    log("configuring openbfme_core (headless, no Godot)")
    _run(["cmake", "-S", str(SRC / "engine"), "-B", str(BUILD), "-G", "Ninja",
          "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
          *flags, *launcher], env=env)
    log("building openbfme_core (first run takes ~20 minutes; cached afterwards)")
    _run(["cmake", "--build", str(BUILD), "--target", "openbfme_core",
          "-j", str(os.cpu_count() or 4)], env=env)
    STAMP.write_text(stamp, encoding="utf-8")


def compile_flags(module: str) -> list[str]:
    """Reuse the compiler flags CMake recorded for the module under test."""

    database = json.loads((BUILD / "compile_commands.json").read_text(encoding="utf-8"))
    wanted = module.replace("\\", "/")
    entry = None
    for item in database:
        if item["file"].replace("\\", "/").endswith(wanted):
            entry = item
            break
    if entry is None:
        entry = next(item for item in database
                     if item["file"].endswith("Common/System/PathIO.cpp"))
    tokens = entry["command"].split()
    flags: list[str] = []
    skip_next = False
    for index, token in enumerate(tokens[1:]):
        if skip_next:
            skip_next = False
            continue
        if token in ("-c", "-o", "-MD", "-MT", "-MF"):
            skip_next = token in ("-o", "-MT", "-MF")
            continue
        if token.endswith(".cpp") or token.endswith(".o"):
            continue
        flags.append(token)
    return flags


def harness_binary(task: dict, module: str) -> Path:
    BIN.mkdir(parents=True, exist_ok=True)
    source = HARNESS_DIR / f"{task['id']}.cpp"
    if not source.exists():
        raise SystemExit(f"missing reference harness {source}")
    binary = BIN / task["id"]
    archives = sorted(str(path) for path in BUILD.rglob("*.a"))
    argv = ["g++", "-std=c++17", "-O1", *compile_flags(module),
            f"-I{HARNESS_DIR}", str(source), "-o", str(binary),
            "-Wl,--start-group", *archives, "-Wl,--end-group", "-lpthread", "-lm", "-ldl"]
    log(f"compiling harness {task['id']}")
    _run(argv, cwd=str(BUILD))
    return binary


def run_harness(binary: Path, args_map: dict) -> list[str]:
    lines = []
    for entry in args_map["api"]:
        for case in range(runner.CASE_COUNT):
            lines.append(runner.wire_line(args_map["case_seed"], entry["name"], case,
                                          entry["args"]))
    result = subprocess.run([str(binary)], input="\n".join(lines) + "\n",
                            capture_output=True, text=True, timeout=1800)
    if result.returncode != 0:
        raise SystemExit(f"reference harness failed: {result.stderr.strip()[:400]}")
    rows = [row for row in result.stdout.splitlines() if row.strip()]
    expected = runner.CASE_COUNT * len(args_map["api"])
    if len(rows) != expected:
        raise SystemExit(f"reference produced {len(rows)} rows, expected {expected}")
    return rows


def upstream_file_digests(config: dict, task: dict) -> dict[str, str]:
    import hashlib

    digests = {}
    for relative in task["upstream_files"]:
        path = SRC / relative
        digests[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
    return digests


def _degeneracy(rows: list[str]) -> dict[str, float]:
    """Share of identical outputs per API function; a flat task proves nothing."""

    buckets: dict[str, list[str]] = {}
    for row in rows:
        fn, _case, encoded = row.split("\t", 2)
        buckets.setdefault(fn, []).append(encoded)
    return {fn: max(values.count(item) for item in set(values)) / len(values)
            for fn, values in buckets.items()}


def mint(task: dict, config: dict, *, check: bool) -> bool:
    args_map = runner.task_args(task)
    binary = harness_binary(task, task["module"])
    rows = run_harness(binary, args_map)
    for fn, share in _degeneracy(rows).items():
        if share > 0.9:
            raise SystemExit(
                f"{task['id']}:{fn} returns the same value for {share:.0%} of cases; "
                "widen the argument kinds or choose a different entry point")
    golden = {
        "schema": runner.SCHEMA,
        "task_id": task["id"],
        "module": task["module"],
        "upstream": {
            "repo": config["repo"],
            "commit": config["commit"],
            "files": upstream_file_digests(config, task),
        },
        "case_seed": task["case_seed"],
        "case_groups": runner.CASE_GROUPS,
        "cases_per_group": runner.CASES_PER_GROUP,
        "examples": runner.example_rows(rows, args_map),
        "digests": runner.group_digests(rows),
    }
    text = json.dumps(golden, indent=2, sort_keys=True) + "\n"
    path = GOLDEN_DIR / f"{task['id']}.json"
    if check:
        current = path.read_text(encoding="utf-8") if path.exists() else ""
        if current != text:
            log(f"DRIFT {task['id']}: committed golden does not match the reference")
            return False
        log(f"ok {task['id']} ({len(rows)} rows)")
        return True
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    log(f"wrote goldens/{task['id']}.json ({len(rows)} rows, {len(text)} bytes)")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", action="append", default=[])
    parser.add_argument("--task-file", action="append", default=[],
                        help="mint a task record that is not in the index yet (authoring aid)")
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    options = parser.parse_args()
    config = upstream_config()
    tasks: list[dict] = []
    for path in options.task_file:
        tasks.append(json.loads(Path(path).read_text(encoding="utf-8")))
    if options.task or options.all:
        index = runner.load_index()
        indexed = runner.task_entries(index)
        if options.task:
            wanted = set(options.task)
            selected = [task for task in indexed if task["id"] in wanted]
            missing = wanted - {task["id"] for task in selected}
            if missing:
                raise SystemExit(f"unknown task(s): {', '.join(sorted(missing))}")
            tasks.extend(selected)
        else:
            tasks.extend(indexed)
    if not tasks:
        parser.error("pass --all, --task <id> or --task-file <path>")
    if not options.skip_build:
        ensure_checkout(config)
        ensure_build(config)
    ok = True
    for task in tasks:
        ok = mint(task, config, check=options.check) and ok
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
