#!/usr/bin/env python3
"""sage-port: verify a Rust port of an openbfme_core module against golden vectors.

The plugin never sees the upstream C++ build, a game binary, or an emulator. A
task declares one upstream module, the Rust API the port must expose, and a case
seed. Case inputs are derived from that seed here; the expected outputs live in
`goldens/<task-id>.json`, minted once by the project owner from the C++ reference
build with `tools/mint_goldens.py`, which imports this file so that both sides
generate byte-identical cases.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import random
import re
import subprocess
import struct
import sys
import tempfile
from pathlib import Path

PLUGIN_ROOT = Path(__file__).resolve().parent
INDEX_PATH = PLUGIN_ROOT / "task-index.json"
PROJECT_ROOT = PLUGIN_ROOT.parents[1]

SCHEMA = 2
CASE_GROUPS = 8
CASES_PER_GROUP = 32
CASE_COUNT = CASE_GROUPS * CASES_PER_GROUP
CASE_NAMES = [f"case-{index:02d}" for index in range(CASE_GROUPS)]
EXAMPLE_CASES = 8
OUTPUT_DIR = "recovery/crates/sage-core/src/port"
GOLDEN_DIR = "goldens"
MAX_OUTPUT_BYTES = 262144

ARG_RUST_TYPES = {
    "bytes": "&[u8]",
    "ascii": "&str",
    "u32": "u32",
    "i32": "i32",
    "u64": "u64",
    "i64": "i64",
    "usize": "usize",
    "f64": "f64",
    "bool": "bool",
    "u32list": "&[u32]",
}
RET_KINDS = {"string", "bytes", "u32", "i32", "u64", "i64", "usize", "f64", "bool", "u32list"}

F64_POOLS = {
    "unit": [0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 0.25, 0.75, 1e-8, -1e-8],
    "angle": [0.0, 0.5, 1.0, 1.5707963267948966, 3.141592653589793, -3.141592653589793,
              6.283185307179586, -1.5707963267948966, 2.5, -2.5, 100.0, -100.0],
    "positive": [0.0, 1.0, 2.0, 0.5, 1e-6, 1e6, 3.5, 123.456, 1e-30, 1e30],
    "wide": [0.0, -0.0, 1.0, -1.0, 1e-300, 1e300, -1e-300, -1e300, 2.5, -2.5,
             float("inf"), float("-inf"), 1.7976931348623157e308, 5e-324],
}


# --------------------------------------------------------------------------- index


def load_index() -> dict:
    index = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
    if index.get("schema") != SCHEMA:
        raise ValueError("task-index.json schema mismatch")
    return index


def task_entries(index: dict | None = None) -> list[dict]:
    return list((index or load_index())["tasks"])


def find_task(task_id: str, index: dict | None = None) -> dict:
    for task in task_entries(index):
        if task["id"] == task_id:
            return task
    raise ValueError(f"unknown task {task_id}")


def task_args(task: dict) -> dict:
    return {
        "module": task["module"],
        "upstream_files": list(task["upstream_files"]),
        "rust_mod": task["rust_mod"],
        "case_seed": task["case_seed"],
        "api": task["api"],
        "golden": f"{GOLDEN_DIR}/{task['id']}.json",
        "task_id": task["id"],
    }


def task_output(task: dict) -> str:
    return f"{OUTPUT_DIR}/{task['rust_mod']}.rs"


def work_plan(index: dict | None = None) -> list[dict]:
    plan = []
    for task in task_entries(index):
        plan.append({
            "id": task["id"],
            "kind": "port-module",
            "args": task_args(task),
            "outputs": [{
                "path": task_output(task),
                "media_type": "text/x-rust",
                "max_bytes": MAX_OUTPUT_BYTES,
            }],
            "weight": int(task["weight"]),
        })
    return plan


# ------------------------------------------------------------------------- cases


def _rng(case_seed: str, fn: str, case: int, position: int) -> random.Random:
    digest = hashlib.sha256(f"{case_seed}|{fn}|{case}|{position}".encode()).digest()
    return random.Random(int.from_bytes(digest, "big"))


def _draw(spec: dict, rng: random.Random):
    kind = spec["kind"]
    if kind == "bytes":
        length = rng.randrange(0, int(spec.get("max_len", 64)) + 1)
        return bytes(rng.randrange(0, 256) for _ in range(length))
    if kind == "ascii":
        length = rng.randrange(0, int(spec.get("max_len", 32)) + 1)
        alphabet = spec.get("alphabet") or (
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-./\\:")
        return "".join(rng.choice(alphabet) for _ in range(length))
    if kind in ("u32", "u64", "usize"):
        high = {"u32": 0xFFFFFFFF, "u64": 0xFFFFFFFFFFFFFFFF, "usize": 0xFFFF}[kind]
        low = int(spec.get("min", 0))
        high = int(spec.get("max", high))
        return rng.randrange(low, high + 1)
    if kind in ("i32", "i64"):
        bound = {"i32": 0x7FFFFFFF, "i64": 0x7FFFFFFFFFFFFFFF}[kind]
        low = int(spec.get("min", -bound - 1))
        high = int(spec.get("max", bound))
        return rng.randrange(low, high + 1)
    if kind == "f64":
        pool = F64_POOLS[spec.get("pool", "unit")]
        if rng.random() < 0.5:
            return rng.choice(pool)
        scale = float(spec.get("scale", 1.0))
        return (rng.random() * 2.0 - 1.0) * scale
    if kind == "bool":
        return rng.random() < 0.5
    if kind == "u32list":
        length = rng.randrange(0, int(spec.get("max_len", 8)) + 1)
        high = int(spec.get("max", 0xFFFFFFFF))
        return [rng.randrange(0, high + 1) for _ in range(length)]
    raise ValueError(f"unknown argument kind {kind}")


def generate_args(case_seed: str, fn: str, case: int, specs: list[dict]) -> list:
    return [_draw(spec, _rng(case_seed, fn, case, position))
            for position, spec in enumerate(specs)]


def wire_token(spec: dict, value) -> str:
    """One argument as the C++ reference harness reads it: `<kind>:<encoding>`."""

    kind = spec["kind"]
    if kind == "bytes":
        return f"bytes:{value.hex()}"
    if kind == "ascii":
        return f"ascii:{value.encode().hex()}"
    if kind == "f64":
        return f"f64:{_f64_bits(value)}"
    if kind == "bool":
        return f"bool:{1 if value else 0}"
    if kind == "u32list":
        body = ",".join(str(item) for item in value) if value else "-"
        return f"u32list:{body}"
    return f"{kind}:{int(value)}"


def wire_line(case_seed: str, fn: str, case: int, specs: list[dict]) -> str:
    """One case as a tab-separated line on the reference harness's stdin."""

    values = generate_args(case_seed, fn, case, specs)
    tokens = [wire_token(spec, value) for spec, value in zip(specs, values)]
    return "\t".join([fn, str(case), *tokens])


def _f64_bits(value: float) -> str:
    return format(int.from_bytes(struct.pack(">d", float(value)), "big"), "016x")


# ------------------------------------------------------------------- rust harness


def _rust_arg(spec: dict, value) -> str:
    kind = spec["kind"]
    if kind == "bytes":
        body = ", ".join(f"0x{byte:02x}" for byte in value)
        return f"&[{body}][..]" if value else "&[][..]"
    if kind == "ascii":
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    if kind == "f64":
        return f"f64::from_bits(0x{_f64_bits(value)}u64)"
    if kind == "bool":
        return "true" if value else "false"
    if kind == "u32list":
        body = ", ".join(f"{item}u32" for item in value)
        return f"&[{body}][..]" if value else "&[][..]"
    if kind in ("i32", "i64"):
        return f"({value}{kind})"
    return f"{value}{kind}"


def _encode_expr(kind: str, expr: str) -> str:
    if kind.startswith("option<"):
        inner = kind[len("option<"):-1]
        return (f"match {expr} {{ None => \"-\".to_string(), "
                f"Some(inner) => {_encode_expr(inner, 'inner')} }}")
    if kind.startswith("result<"):
        inner = kind[len("result<"):-1]
        return (f"match {expr} {{ Err(_) => \"!\".to_string(), "
                f"Ok(inner) => {_encode_expr(inner, 'inner')} }}")
    if kind == "string":
        return f"enc_bytes({expr}.as_bytes())"
    if kind == "bytes":
        return f"enc_bytes(AsRef::<[u8]>::as_ref(&{expr}))"
    if kind == "f64":
        return f"format!(\"{{:016x}}\", {expr}.to_bits())"
    if kind == "bool":
        return f"(if {expr} {{ \"1\" }} else {{ \"0\" }}).to_string()"
    if kind == "u32list":
        return f"enc_u32list(AsRef::<[u32]>::as_ref(&{expr}))"
    if kind in RET_KINDS:
        return f"{expr}.to_string()"
    raise ValueError(f"unknown return kind {kind}")


HARNESS_PRELUDE = """#![forbid(unsafe_code)]
#![allow(dead_code, unused_imports, clippy::all)]

mod candidate;

fn enc_bytes(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn enc_u32list(data: &[u32]) -> String {
    if data.is_empty() {
        return "-".to_string();
    }
    data.iter().map(|value| value.to_string()).collect::<Vec<_>>().join(",")
}

fn main() {
    let mut rows: Vec<String> = Vec::new();
"""


def rust_harness(task_args_map: dict) -> str:
    lines = [HARNESS_PRELUDE]
    for entry in task_args_map["api"]:
        fn = entry["name"]
        specs = entry["args"]
        for case in range(CASE_COUNT):
            values = generate_args(task_args_map["case_seed"], fn, case, specs)
            call_args = ", ".join(_rust_arg(spec, value) for spec, value in zip(specs, values))
            lines.append(f"    let out = candidate::{fn}({call_args});")
            lines.append(f"    rows.push(format!(\"{fn}\\t{case}\\t{{}}\", "
                         f"{_encode_expr(entry['ret'], 'out')}));")
    lines.append("    println!(\"{}\", rows.join(\"\\n\"));")
    lines.append("}")
    return "\n".join(lines) + "\n"


def _rustc() -> str:
    for candidate in ("/usr/local/bin/rustc", "/usr/bin/rustc"):
        if Path(candidate).exists():
            return candidate
    return "rustc"


def run_candidate(source: str, args_map: dict) -> list[str]:
    """Compile the submitted module with the generated harness and collect rows."""

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "candidate.rs").write_text(source, encoding="utf-8")
        (root / "harness.rs").write_text(rust_harness(args_map), encoding="utf-8")
        build = subprocess.run(
            [_rustc(), "--edition=2021", "-C", "opt-level=1", "--crate-name", "harness",
             "harness.rs", "-o", "harness"],
            cwd=root, capture_output=True, text=True, timeout=600)
        if build.returncode != 0:
            raise RuntimeError("candidate does not compile: "
                               + build.stderr.strip().splitlines()[-1][:200])
        first = subprocess.run(["./harness"], cwd=root, capture_output=True, text=True,
                               timeout=300)
        if first.returncode != 0:
            raise RuntimeError("candidate panicked or aborted while running cases")
        second = subprocess.run(["./harness"], cwd=root, capture_output=True, text=True,
                                timeout=300)
        if second.stdout != first.stdout:
            raise RuntimeError("candidate is not deterministic across runs")
    rows = [line for line in first.stdout.splitlines() if line.strip()]
    expected = CASE_COUNT * len(args_map["api"])
    if len(rows) != expected:
        raise RuntimeError(f"expected {expected} rows, candidate produced {len(rows)}")
    return rows


def group_digests(rows: list[str]) -> dict[str, str]:
    groups: dict[str, list[str]] = {name: [] for name in CASE_NAMES}
    for row in rows:
        fn, case, _ = row.split("\t", 2)
        index = int(case) // CASES_PER_GROUP
        groups[CASE_NAMES[index]].append(row)
    digests = {}
    for name, bucket in groups.items():
        bucket.sort(key=lambda row: (row.split("\t", 2)[0], int(row.split("\t", 2)[1])))
        payload = "\n".join(bucket) + "\n"
        digests[name] = hashlib.sha256(payload.encode()).hexdigest()
    return digests


def example_rows(rows: list[str], args_map: dict) -> list[dict]:
    index = {(row.split("\t", 2)[0], int(row.split("\t", 2)[1])): row.split("\t", 2)[2]
             for row in rows}
    examples = []
    for entry in args_map["api"]:
        for case in range(EXAMPLE_CASES):
            values = generate_args(args_map["case_seed"], entry["name"], case, entry["args"])
            examples.append({
                "fn": entry["name"],
                "case": case,
                "args": [wire_token(spec, value) for spec, value in zip(entry["args"], values)],
                "out": index[(entry["name"], case)],
            })
    return examples


# ------------------------------------------------------------------------ verdict


def compare(rows: list[str], golden: dict) -> tuple[bool, dict, dict]:
    digests = group_digests(rows)
    expected = golden["digests"]
    diverged = [name for name in CASE_NAMES if digests.get(name) != expected.get(name)]
    row_index = {(row.split("\t", 2)[0], int(row.split("\t", 2)[1])): row.split("\t", 2)[2]
                 for row in rows}
    bad_examples = [f"{item['fn']}#{item['case']}" for item in golden.get("examples", [])
                    if row_index.get((item["fn"], item["case"])) != item["out"]]
    evidence = {
        "groups": len(CASE_NAMES),
        "groups_matched": len(CASE_NAMES) - len(diverged),
        "cases": CASE_COUNT,
        "first_divergent_group": diverged[0] if diverged else "",
        "examples_failed": len(bad_examples),
        "first_failed_example": bad_examples[0] if bad_examples else "",
    }
    return (not diverged and not bad_examples), digests, evidence


def load_golden(root: Path, args_map: dict) -> dict:
    path = root / args_map["golden"]
    golden = json.loads(path.read_text(encoding="utf-8"))
    if golden.get("schema") != SCHEMA:
        raise RuntimeError("golden file schema mismatch")
    if golden.get("case_seed") != args_map["case_seed"]:
        raise RuntimeError("golden file does not belong to this task")
    return golden


# ----------------------------------------------------------------------- commands


def stub_source(task: dict) -> str:
    lines = [
        f"//! Port of `{task['module']}` from openbfme_core.",
        "//!",
        "//! Upstream sources:",
    ]
    lines += [f"//!   - `{path}`" for path in task["upstream_files"]]
    lines += [
        "//!",
        "//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,",
        "//! no I/O, no clock, no randomness, no global mutable state. The verifier",
        "//! compiles this file on its own, so it may not reference other modules.",
        "",
        "#![allow(unused_variables)]",
        "",
    ]
    for entry in task["api"]:
        lines.append(f"{entry['rust']} {{")
        lines.append(f"    unimplemented!(\"port {task['module']}: {entry['name']}\")")
        lines.append("}")
        lines.append("")
    return "\n".join(lines)


def cmd_work_plan(_payload: dict) -> dict:
    return {"tasks": work_plan()}


def cmd_run_task(payload: dict) -> dict:
    args = payload.get("args") or {}
    task = find_task(args["task_id"])
    outputs = payload.get("outputs") or [task_output(task)]
    path = outputs[0] if isinstance(outputs[0], str) else outputs[0]["path"]
    if path != task_output(task):
        raise ValueError("task output path does not match the task record")
    return {"files": {path: stub_source(task)}}


def cmd_verify(payload: dict) -> dict:
    args = payload.get("args") or {}
    files = payload.get("files") or {}
    output = task_output(find_task(args["task_id"]))
    source = files.get(output)
    if source is None:
        return {"pass": False, "evidence": {"error": "missing-output"}}
    try:
        rows = run_candidate(source, args)
        golden = load_golden(PROJECT_ROOT, args)
        passed, _digests, evidence = compare(rows, golden)
    except Exception as exc:  # noqa: BLE001 - reported as a verdict, never a crash
        return {"pass": False, "evidence": {"error": type(exc).__name__, "detail": str(exc)[:200]}}
    return {"pass": passed, "evidence": evidence}


def cmd_selftest(_payload: dict) -> dict:
    index = load_index()
    checks = 0
    for task in task_entries(index):
        checks += 1
        if not re.fullmatch(r"[a-z0-9][a-z0-9_]*", task["rust_mod"]):
            raise ValueError(f"{task['id']}: rust_mod must be snake_case")
        if not task["api"]:
            raise ValueError(f"{task['id']}: no API declared")
        for entry in task["api"]:
            signature = entry["rust"]
            if not signature.startswith(f"pub fn {entry['name']}("):
                raise ValueError(f"{task['id']}: signature does not declare {entry['name']}")
            declared = [part.split(":", 1)[1].strip()
                        for part in _signature_params(signature)]
            expected = [ARG_RUST_TYPES[spec["kind"]] for spec in entry["args"]]
            if declared != expected:
                raise ValueError(
                    f"{task['id']}:{entry['name']} takes {declared}, cases supply {expected}")
            _encode_expr(entry["ret"], "out")
        golden_path = PROJECT_ROOT / f"{GOLDEN_DIR}/{task['id']}.json"
        if not golden_path.exists():
            raise ValueError(f"{task['id']}: {golden_path.name} has not been minted")
        golden = json.loads(golden_path.read_text(encoding="utf-8"))
        if sorted(golden["digests"]) != CASE_NAMES:
            raise ValueError(f"{task['id']}: golden digests are incomplete")
        if golden["case_seed"] != task["case_seed"]:
            raise ValueError(f"{task['id']}: golden seed does not match the task")
        harness = PROJECT_ROOT / "reference" / "harness" / f"{task['id']}.cpp"
        if not harness.exists():
            raise ValueError(f"{task['id']}: reference harness is missing")
    return {"ok": True, "tasks": checks, "cases_per_task": CASE_COUNT}


def _signature_params(signature: str) -> list[str]:
    inner = signature[signature.index("(") + 1: signature.rindex(")")]
    params, depth, current = [], 0, ""
    for char in inner:
        if char in "<([":
            depth += 1
        elif char in ">)]":
            depth -= 1
        if char == "," and depth == 0:
            params.append(current)
            current = ""
            continue
        current += char
    if current.strip():
        params.append(current)
    return [param.strip() for param in params if param.strip()]


COMMANDS = {
    "work-plan": cmd_work_plan,
    "run-task": cmd_run_task,
    "verify": cmd_verify,
    "selftest": cmd_selftest,
}


# -------------------------------------------------------------------------- guest


def cmd_guest(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", required=True)
    parser.add_argument("--input")
    parser.add_argument("--target")
    options = parser.parse_args(argv)
    job = json.loads(Path(options.job).read_text(encoding="utf-8"))
    payload = job.get("payload", job)
    args = payload.get("args") or job.get("args") or {}
    verdict = {"status": "rejected", "outputs": {}, "probes": {}, "diagnostic_code": 10}
    try:
        if options.input is None:
            raise RuntimeError("no candidate input")
        root = Path(options.input)
        task = find_task(args["task_id"])
        source = (root / task_output(task)).read_text(encoding="utf-8")
        rows = run_candidate(source, args)
        golden = load_golden(root, args)
        passed, digests, evidence = compare(rows, golden)
        verdict["outputs"] = digests
        verdict["probes"] = {key: str(value)[:256] for key, value in evidence.items()}
        if passed:
            verdict["status"] = "accepted"
            verdict["diagnostic_code"] = 0
    except Exception as exc:  # noqa: BLE001 - a rejection, never a crash
        verdict["probes"] = {"error": type(exc).__name__, "detail": str(exc)[:256]}
    print(json.dumps(verdict, sort_keys=True, separators=(",", ":")))
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: runner.py <command>", file=sys.stderr)
        return 2
    command = sys.argv[1]
    if command == "guest":
        return cmd_guest(sys.argv[2:])
    handler = COMMANDS.get(command)
    if handler is None:
        print(f"unknown command {command}", file=sys.stderr)
        return 2
    payload = json.loads(sys.stdin.read() or "{}")
    result = handler(payload)
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
