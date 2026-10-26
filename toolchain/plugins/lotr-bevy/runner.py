#!/usr/bin/env python3
"""BFME II Rust recovery workflow and separated behavioral verifier."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

PLUGIN_ROOT = Path(__file__).resolve().parent
INDEX_PATH = PLUGIN_ROOT / "task-index.json"
TARGET_SHA256 = "1aa5dca6595820e508df31352275e533cce93dd3be7b0ed7528530413ce99c5d"
CASE_COUNT = 8
BATCH_SIZE = 12
SCRATCH = 0x2000_0000
STACK_BASE = 0x1000_0000
STACK_SIZE = 1 << 20
SENTINEL = 0x0DEAD000
MAX_INSN = 100_000
POOLS = {
    "i32": (0, 1, -1, 7, -7, 2**31 - 1, -(2**31), 65536),
    "u32": (0, 1, 2**32 - 1, 42, 65535, 65536, 0x80000000, 7),
    "bool": (0, 1),
    "enum4": (0, 1, 2, 3, 5, 8, 16, 0x7FFFFFFF),
    "f32": (0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2.5, -7.25),
    "f64": (0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2.5, -7.25),
    "c8": (0, 1, 65, 122, 255),
    "char": (0, 1, 65, 122, 255),
}


class WorkflowError(RuntimeError):
    pass


def load_index() -> dict[str, Any]:
    document = json.loads(INDEX_PATH.read_text())
    if document.get("schema") != 1 or document.get("target_sha256") != TARGET_SHA256:
        raise WorkflowError("task index does not match the pinned target")
    functions = document.get("functions")
    if not isinstance(functions, list) or not functions:
        raise WorkflowError("task index has no functions")
    return document


def public_function(entry: dict[str, Any]) -> dict[str, Any]:
    return {key: entry[key] for key in
            ("addr", "size", "module", "name", "source", "sig", "decomp_sha256")}


def _rustc() -> str:
    """Prefer the real toolchain rustc; the rustup shim writes into HOME and
    spawns one thread per visible CPU, both hostile to read-only rlimits."""
    for candidate in ("/usr/bin/rustc", "/usr/local/bin/rustc"):
        if os.path.isfile(candidate):
            return candidate
    found = subprocess.run(["sh", "-c", "command -v rustc"],
                           capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, check=False)
    return found.stdout.strip() or "rustc"


def task_batches() -> list[dict[str, Any]]:
    functions = [public_function(item) for item in load_index()["functions"]]
    tasks = []
    for number, start in enumerate(range(0, len(functions), BATCH_SIZE)):
        batch = functions[start:start + BATCH_SIZE]
        task_id = f"port-{number:04d}"
        rust_module = task_id.replace("-", "_")
        output = f"recovery/crates/sage-core/src/recomp_tasks/{rust_module}.rs"
        tasks.append({
            "id": task_id,
            "kind": "t2-rust-differential",
            "args": {"output": output, "functions": batch},
            "outputs": [{"path": output, "media_type": "text/x-rust",
                         "max_bytes": 262144}],
            "weight": sum(item["size"] for item in batch),
        })
    return tasks


def cmd_detect(payload: dict[str, Any]) -> dict[str, Any]:
    target = Path(payload["binary"])
    digest = hashlib.sha256(target.read_bytes()).hexdigest()
    return {"format": "pe32", "engine": "sage", "tier": "t2-recomp",
            "target_matches": digest == TARGET_SHA256}


def cmd_work_plan(_payload: dict[str, Any]) -> dict[str, Any]:
    return {"tasks": task_batches()}


def _stub(task_args: dict[str, Any]) -> str:
    lines = [
        "//! Reimplement the original x86 functions listed below.",
        "//! Inputs use raw ABI bits: f32 in low 32 bits, f64 in all 64 bits.",
        "//! Read reference/decomp/<address>.c; do not copy target bytes.",
        "",
        "use super::Observed;",
        "",
    ]
    for item in task_args["functions"]:
        lines.append(f"// {item['addr']} {item['name']}")
        lines.append(f"// signature: {json.dumps(item['sig'], sort_keys=True)}")
        lines.append(f"// reference: reference/decomp/{item['addr']}.c")
    lines.extend([
        "",
        "pub fn call(address: u32, args: &[u64], scratch: &mut [u8; 64])",
        "    -> Option<Observed>",
        "{",
        "    let _ = (address, args, scratch);",
        "    None",
        "}",
        "",
    ])
    return "\n".join(lines)


def cmd_run_task(payload: dict[str, Any]) -> dict[str, Any]:
    task_args = payload.get("args") or {}
    outputs = payload.get("outputs") or []
    if len(outputs) != 1 or outputs[0].get("path") != task_args.get("output"):
        raise WorkflowError("task output does not match its trusted record")
    return {"files": {task_args["output"]: _stub(task_args)}}


def _bits(kind: str, value: int | float) -> int:
    if kind == "f32":
        return struct.unpack("<I", struct.pack("<f", value))[0]
    if kind == "f64":
        return struct.unpack("<Q", struct.pack("<d", value))[0]
    return int(value) & (0xFF if kind in {"c8", "char"} else 0xFFFFFFFF)


def case_inputs(addr: str, sig: dict[str, Any], case: int) -> list[int]:
    result = []
    for position, kind in enumerate(sig["args"]):
        pool = POOLS[kind]
        seed = hashlib.sha256(f"{addr}:{case}:{position}".encode()).digest()
        result.append(_bits(kind, pool[int.from_bytes(seed[:4], "little") % len(pool)]))
    return result


def initial_scratch(addr: str, case: int) -> bytes:
    choices = (0, 1, 7, 0xFFFFFFFF, 0x3F800000, 0xBF800000)
    words = []
    for position in range(16):
        seed = hashlib.sha256(f"{addr}:{case}:scratch:{position}".encode()).digest()
        words.append(choices[int.from_bytes(seed[:4], "little") % len(choices)])
    return b"".join(struct.pack("<I", value) for value in words)


def _rust_literal(path: Path) -> str:
    return '"' + os.fspath(path).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _candidate_harness(source: Path, functions: list[dict[str, Any]]) -> str:
    calls = []
    for case in range(CASE_COUNT):
        for entry in functions:
            args = ", ".join(f"{value}u64" for value in case_inputs(entry["addr"], entry["sig"], case))
            scratch = ", ".join(str(value) for value in initial_scratch(entry["addr"], case))
            calls.append(f"""
    {{
        let mut scratch: [u8; 64] = [{scratch}];
        let observed = candidate::call({int(entry['addr'], 16)}u32, &[{args}], &mut scratch)
            .unwrap_or_else(|| {{ eprintln!("unimplemented {entry['addr']}"); std::process::exit(2); }});
        println!("{entry['addr']}|{case}|{{}}|{{}}|{{}}",
            observed.eax.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
            observed.double_bits.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
            hex(&scratch));
    }}""")
    return f"""use std::fmt::Write as _;

#[derive(Clone, Copy, Debug)]
pub struct Observed {{
    pub eax: Option<u32>,
    pub double_bits: Option<u64>,
}}

mod candidate {{ include!({_rust_literal(source)}); }}

fn hex(bytes: &[u8]) -> String {{
    let mut out = String::with_capacity(bytes.len() * 2);
    for value in bytes {{ write!(&mut out, "{{:02x}}", value).unwrap(); }}
    out
}}

fn main() {{
{''.join(calls)}
}}
"""


def _compile_candidate(source_text: str, functions: list[dict[str, Any]], *, run: bool) -> list[dict[str, Any]]:
    with tempfile.TemporaryDirectory(prefix="recomp-candidate-") as temporary:
        root = Path(temporary)
        source = root / "candidate.rs"
        harness = root / "harness.rs"
        executable = root / "candidate"
        source.write_text(source_text)
        harness.write_text(_candidate_harness(source, functions))
        built = subprocess.run(
            [_rustc(), "--edition=2021", "-C", "opt-level=1", os.fspath(harness),
             "-o", os.fspath(executable)],
            capture_output=True, text=True, timeout=240, check=False,
            stdin=subprocess.DEVNULL,
        )
        if built.returncode != 0:
            raise WorkflowError("candidate Rust module does not compile")
        if not run:
            return []
        completed = subprocess.run(
            [os.fspath(executable)], capture_output=True, text=True,
            timeout=120, check=False, stdin=subprocess.DEVNULL,
        )
        if completed.returncode != 0:
            raise WorkflowError("candidate Rust module did not implement every function")
        observations = []
        for line in completed.stdout.splitlines():
            parts = line.split("|")
            if len(parts) != 5:
                raise WorkflowError("candidate harness emitted an invalid result")
            addr, case, eax, double_bits, scratch = parts
            observations.append({
                "addr": addr,
                "case": int(case),
                "eax": None if eax == "-" else int(eax),
                "double_bits": None if double_bits == "-" else int(double_bits),
                "scratch": scratch,
            })
        expected = len(functions) * CASE_COUNT
        if len(observations) != expected:
            raise WorkflowError(f"candidate emitted {len(observations)} rows; expected {expected}")
        return observations


def _encoded(kind: str, raw: int) -> bytes:
    if kind == "f64":
        return struct.pack("<Q", raw & 0xFFFFFFFFFFFFFFFF)
    if kind in {"c8", "char"}:
        return struct.pack("<I", raw & 0xFF)
    return struct.pack("<I", raw & 0xFFFFFFFF)


def _load_image(data: bytes) -> tuple[int, int, list[tuple[str, int, int, int, int]]]:
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise WorkflowError("target is not a PE image")
    coff = pe + 4
    sections = struct.unpack_from("<H", data, coff + 2)[0]
    optional_size = struct.unpack_from("<H", data, coff + 16)[0]
    optional = coff + 20
    image_base = struct.unpack_from("<I", data, optional + 28)[0]
    image_size = struct.unpack_from("<I", data, optional + 56)[0]
    parsed = []
    for number in range(sections):
        offset = optional + optional_size + number * 40
        name = data[offset:offset + 8].rstrip(b"\0").decode("latin1")
        virtual_size, va, raw_size, raw_offset = struct.unpack_from("<IIII", data, offset + 8)
        parsed.append((name, va, virtual_size, raw_size, raw_offset))
    return image_base, image_size, parsed


def _oracle_observations(target: Path, functions: list[dict[str, Any]]) -> list[dict[str, Any]]:
    from unicorn import Uc, UcError, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_UNMAPPED, UC_MODE_32, UC_PROT_ALL
    from unicorn.x86_const import (UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX,
        UC_X86_REG_ECX, UC_X86_REG_EDI, UC_X86_REG_EDX, UC_X86_REG_EFLAGS,
        UC_X86_REG_ESI, UC_X86_REG_ESP)

    data = target.read_bytes()
    if hashlib.sha256(data).hexdigest() != TARGET_SHA256:
        raise WorkflowError("oracle target hash does not match the project")
    image_base, image_size, sections = _load_image(data)
    machine = Uc(UC_ARCH_X86, UC_MODE_32)
    machine.mem_map(image_base, (image_size + 0xFFF) & ~0xFFF, UC_PROT_ALL)
    for _name, va, _virtual_size, raw_size, raw_offset in sections:
        if raw_size:
            machine.mem_write(image_base + va, data[raw_offset:raw_offset + raw_size])
    machine.mem_map(STACK_BASE, STACK_SIZE, UC_PROT_ALL)
    machine.mem_map(SCRATCH, 0x10000, UC_PROT_ALL)
    machine.mem_map(SENTINEL, 0x1000, UC_PROT_ALL)
    machine.mem_write(SENTINEL, bytes([0xDD, 0x24, 0x24]))
    machine.mem_map(0, 0x1000, UC_PROT_ALL)
    machine.mem_write(0, bytes([0xFF]) * 8)
    state: dict[str, Any] = {"fail": None, "count": 0}

    def unmapped(_machine, access, address, _size, _value, _user):
        state["fail"] = (access, address)
        return False

    def instruction(_machine, _address, _size, _user):
        state["count"] += 1
        if state["count"] > MAX_INSN:
            machine.emu_stop()

    machine.hook_add(UC_HOOK_MEM_UNMAPPED, unmapped)
    machine.hook_add(UC_HOOK_CODE, instruction)
    observations = []
    for case in range(CASE_COUNT):
        for entry in functions:
            sig = entry["sig"]
            raw = case_inputs(entry["addr"], sig, case)
            scratch_in = initial_scratch(entry["addr"], case)
            machine.mem_write(SCRATCH, scratch_in + bytes(0x10000 - 64))
            stack = STACK_BASE + STACK_SIZE // 2
            machine.mem_write(stack, struct.pack("<I", SENTINEL))
            cursor = stack + 4
            for kind, value in zip(sig["args"], raw):
                argument = _encoded(kind, value)
                machine.mem_write(cursor, argument)
                cursor += len(argument)
            for register in (UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_EDX,
                             UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EFLAGS):
                machine.reg_write(register, 0)
            machine.reg_write(UC_X86_REG_ESP, stack)
            machine.reg_write(UC_X86_REG_EBP, stack - 4)
            machine.reg_write(UC_X86_REG_ECX, SCRATCH)
            state["fail"], state["count"] = None, 0
            until = SENTINEL + 3 if sig["want_float"] else SENTINEL
            try:
                machine.emu_start(int(entry["addr"], 16), until)
            except UcError as exc:
                raise WorkflowError(f"oracle could not execute {entry['addr']}") from exc
            if state["fail"] is not None or state["count"] > MAX_INSN:
                raise WorkflowError(f"oracle needs unsupported state for {entry['addr']}")
            eax = None
            double_bits = None
            if sig["want_float"]:
                double_bits = struct.unpack("<Q", bytes(machine.mem_read(stack + 4, 8)))[0]
            elif sig["ret"] != "void":
                eax = machine.reg_read(UC_X86_REG_EAX) & 0xFFFFFFFF
            observations.append({
                "addr": entry["addr"], "case": case, "eax": eax,
                "double_bits": double_bits,
                "scratch": bytes(machine.mem_read(SCRATCH, 64)).hex(),
            })
    return observations


def _case_digests(observations: list[dict[str, Any]]) -> dict[str, str]:
    outputs = {}
    for case in range(CASE_COUNT):
        rows = [{key: item[key] for key in ("addr", "eax", "double_bits", "scratch")}
                for item in observations if item["case"] == case]
        rows.sort(key=lambda item: item["addr"])
        canonical = json.dumps(rows, sort_keys=True, separators=(",", ":")).encode()
        outputs[f"case-{case:02d}"] = hashlib.sha256(canonical).hexdigest()
    return outputs


def _source_from_payload(payload: dict[str, Any]) -> tuple[str, list[dict[str, Any]]]:
    task_args = payload.get("args") or {}
    files = payload.get("files") or {}
    output = task_args.get("output")
    if not isinstance(output, str) or not isinstance(files.get(output), str):
        raise WorkflowError("candidate output file is missing")
    return files[output], task_args["functions"]


def cmd_verify(payload: dict[str, Any]) -> dict[str, Any]:
    try:
        source, functions = _source_from_payload(payload)
        candidate = _case_digests(_compile_candidate(source, functions, run=True))
        oracle = _case_digests(_oracle_observations(Path(payload["binary"]), functions))
        passed = candidate == oracle
        return {"pass": passed, "evidence": {"cases": CASE_COUNT,
                                                "functions": len(functions),
                                                "matched": sum(candidate.get(k) == v for k, v in oracle.items())}}
    except (OSError, subprocess.SubprocessError, WorkflowError, ValueError) as exc:
        return {"pass": False, "evidence": {"error": type(exc).__name__}}


def cmd_selftest(_payload: dict[str, Any]) -> dict[str, Any]:
    index = load_index()
    functions = index["functions"]
    if int(index.get("case_count", 0)) != CASE_COUNT:
        raise WorkflowError("task index case count is stale")
    for entry in functions:
        if len(entry.get("proof") or []) != CASE_COUNT:
            raise WorkflowError(f"{entry.get('addr')} lacks measured oracle proof")
        if len(entry.get("decomp_sha256", "")) != 64:
            raise WorkflowError(f"{entry.get('addr')} lacks a decomp hash")
    return {"ok": True, "tests": 3, "functions": len(functions), "tasks": len(task_batches())}


def _guest_payload(document: dict[str, Any]) -> tuple[str, dict[str, Any]]:
    operation = document.get("operation")
    payload = document.get("payload")
    if not isinstance(operation, str) or not isinstance(payload, dict):
        raise WorkflowError("guest job is malformed")
    return operation, payload


def _candidate_source(input_root: Path, payload: dict[str, Any]) -> tuple[str, list[dict[str, Any]]]:
    task_args = payload.get("args") or {}
    relative = task_args.get("output")
    if not isinstance(relative, str):
        raise WorkflowError("guest job has no candidate output path")
    root = input_root.resolve()
    path = (root / relative).resolve()
    if root not in path.parents or not path.is_file():
        raise WorkflowError("candidate output path is outside the input tree or missing")
    return path.read_text(), task_args["functions"]


def cmd_guest(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", required=True)
    parser.add_argument("--input")
    parser.add_argument("--target")
    args = parser.parse_args(argv)
    try:
        operation, payload = _guest_payload(json.loads(Path(args.job).read_text()))
        functions = (payload.get("args") or {}).get("functions")
        if not isinstance(functions, list) or not functions:
            raise WorkflowError("guest job has no trusted function batch")
        if operation in {"candidate-build", "candidate-run"}:
            if not args.input:
                raise WorkflowError("candidate operation has no input drive")
            source, functions = _candidate_source(Path(args.input), payload)
            observations = _compile_candidate(source, functions, run=operation == "candidate-run")
            outputs = _case_digests(observations) if observations else {}
        elif operation == "oracle-run":
            if not args.target:
                raise WorkflowError("oracle operation has no target device")
            outputs = _case_digests(_oracle_observations(Path(args.target), functions))
        else:
            raise WorkflowError(f"unsupported guest operation {operation}")
        result = {"status": "accepted", "outputs": outputs, "diagnostic_code": 0}
    except (OSError, subprocess.SubprocessError, WorkflowError, ValueError) as exc:
        result = {"status": "rejected", "outputs": {}, "diagnostic_code": 10,
                  "probes": {"error": type(exc).__name__}}
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


COMMANDS = {
    "detect": cmd_detect,
    "work-plan": cmd_work_plan,
    "run-task": cmd_run_task,
    "verify": cmd_verify,
    "selftest": cmd_selftest,
}


def main() -> int:
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "guest":
        return cmd_guest(sys.argv[2:])
    function = COMMANDS.get(command)
    if function is None:
        print(json.dumps({"error": f"unknown command {command!r}"}))
        return 3
    try:
        payload = json.load(sys.stdin)
        result = function(payload)
    except Exception as exc:  # protocol surface: errors are structured and bounded
        print(json.dumps({"error": f"{type(exc).__name__}: {exc}"}))
        return 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
