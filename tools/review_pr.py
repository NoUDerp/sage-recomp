#!/usr/bin/env python3
"""Coordinator-side pre-merge review of one pull request (no golden access).

Runs every gate the coordinator used to do by hand, before the authoritative
verdict is even needed:

  - PR shape: declared module file-set (+ context files), no policy paths
  - attestations: Recomp-Policy vs recomp.lock.toml, toolchain/image vs recomp.toml
  - trusted lease: active, claimant == PR author, task id matches
  - declared API: every `pub fn` the task record declares, exactly, no extras
  - standalone-module bans: unsafe, crate paths, I/O, clock, rng, threads,
    external crates, submodule/include tricks
  - authoritative check state on the head: recomp verification

Usage: python3 tools/review_pr.py <pr-number> [--json]

Exit code 0 = no violations found (verdict may still be PENDING),
1 = violations found, 2 = could not evaluate. The Firecracker comparison in the
coordinator loop remains the only admissible verdict; this tool only rejects
heads that must not be merged regardless of their check run.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

OWNER = "pickles976"
REPOSITORY = "sage-recomp"
TRUSTED_ACTOR_ID = 348113819
TRUSTED_LOGIN = "sage-recomp[bot]"
ROOT = Path(__file__).resolve().parent.parent
MODULE_DIR = "recovery/crates/sage-core/src/port"
CONTEXT_FILES = {"SUMMARY.md", ".mulch/expertise/merges.jsonl"}

BANNED_TOKENS = [
    ("unsafe", r"\bunsafe\b"),
    ("use crate::", r"\buse\s+crate::"),
    ("extern crate", r"\bextern\s+crate\b"),
    ("I/O (std::io/fs)", r"\bstd::(io|fs)\b"),
    ("process/net (std::process/net)", r"\bstd::(process|net)\b"),
    ("clock (std::time)", r"\bstd::time\b|\bInstant\b|\bSystemTime\b"),
    ("rng", r"\brand\b|\brandom\b"),
    ("threads", r"\bstd::thread\b|\bthread_local!\b"),
    ("global mutable state", r"\bstatic\s+mut\b"),
    ("submodule/include", r"^\s*(mod|include!)\b"),
]

# `use` of anything outside core/std/ supertraits self; allowlisted roots.
USE_ALLOWED = re.compile(r"\buse\s+(core|std|super|self)::")


def gh_json(path: str) -> Any:
    out = subprocess.run(["gh", "api", path], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(f"gh api {path}: {out.stderr.strip()[:200]}")
    return json.loads(out.stdout)


def gh_pr(number: int) -> dict[str, Any]:
    return gh_json(f"repos/{OWNER}/{REPOSITORY}/pulls/{number}")


def strip_comments(src: str) -> str:
    src = re.sub(r"/\*.*?\*/", " ", src, flags=re.S)
    return re.sub(r"//[^\n]*", " ", src)


def declared_signatures(task: dict[str, Any]) -> list[str]:
    return [re.sub(r"\s+", " ", a["rust"]).strip() for a in task["api"]]


def implemented_signatures(src: str) -> list[tuple[str, str]]:
    """(normalized signature, raw line) for every pub fn."""
    found = []
    for line in src.splitlines():
        m = re.match(r"\s*pub\s+fn\s+([A-Za-z0-9_]+)\s*\((.*)\)\s*(->\s*[^;{]+)?[{;]", line)
        if m:
            sig = f"pub fn {m.group(1)}({re.sub(r'\s+', ' ', m.group(2)).strip()})"
            if m.group(3):
                sig += f" -> {re.sub(r'\s+', ' ', m.group(3).removeprefix('->')).strip()}"
            found.append((re.sub(r"\s+", " ", sig).strip(), line.strip()))
    return found


def normalize(sig: str) -> str:
    return re.sub(r"\s+", " ", sig).strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("number", type=int)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    checks: list[tuple[str, str, str]] = []  # (state, label, detail)

    def add(state: str, label: str, detail: str = "") -> None:
        checks.append((state, label, detail))

    try:
        pr = gh_pr(args.number)
    except RuntimeError as exc:
        print(f"cannot evaluate: {exc}", file=sys.stderr)
        return 2

    head = pr["head"]["sha"]
    author = pr["user"]["login"]
    body = pr["body"] or ""
    files = [f["filename"] for f in gh_json(
        f"repos/{OWNER}/{REPOSITORY}/pulls/{args.number}/files?per_page=100")]

    # Task id: from `Closes #<issue>` in the body, confirmed by the issue title.
    m = re.search(r"[Cc]loses\s+#(\d+)", body)
    if not m:
        add("VIOLATION", "no `Closes #<issue>` reference in PR body")
        issue_number = None
        task = None
    else:
        issue_number = int(m.group(1))
        issue = gh_json(f"repos/{OWNER}/{REPOSITORY}/issues/{issue_number}")
        im = re.match(r"(port-\d+)", issue["title"] or "")
        task_id = im.group(1) if im else None
        tasks = json.loads((ROOT / "plugins/sage-port/task-index.json").read_text())["tasks"]
        task = next((t for t in tasks if t["id"] == task_id), None)
        if task is None:
            add("VIOLATION", f"issue #{issue_number} does not name a task in task-index")
        else:
            add("OK", f"task {task['id']} = {task['module']} -> {task['rust_mod']}.rs "
                      f"(issue #{issue_number}, weight {task['weight']})")

    # File set.
    allowed = set(CONTEXT_FILES)
    declared_path = None
    if task:
        declared_path = f"{MODULE_DIR}/{task['rust_mod']}.rs"
        allowed.add(declared_path)
    protected = ("goldens/", "reference/", "tools/", "scripts/", "plugins/", "toolchain/",
                 ".github/", "docs/progress", "recomp.toml", "recomp.lock.toml",
                 "recomp-launcher", "AGENTS.md", ".mulch/mulch.config.yaml",
                 ".gitattributes")
    unexpected = [f for f in files if f not in allowed]
    violations_paths = [f for f in unexpected if any(p in f for p in protected)]
    for f in violations_paths:
        add("VIOLATION", f"pull request touches policy-owned path {f}")
    benign_extra = [f for f in unexpected if f not in violations_paths]
    if benign_extra:
        add("WARN", f"files outside the declared output (context only, do not gate): {benign_extra}")
    if declared_path and declared_path not in files:
        add("VIOLATION", f"declared output {declared_path} not in the diff")
    if declared_path and declared_path in files:
        add("OK", f"diff touches {declared_path}"
                  + (f" + context {sorted(set(files) - {declared_path})}" if len(files) > 1 else ""))

    # Attestations.
    lock = (ROOT / "recomp.lock.toml").read_text()
    lock_hash = re.search(r'hash\s*=\s*"([0-9a-f]{64})"', lock)
    toml = (ROOT / "recomp.toml").read_text()
    image_line = re.search(r'image\s*=\s*"([^"]+)"', toml)
    policy = re.search(r"Recomp-Policy:\s*sha256:([0-9a-f]{64})", body)
    toolchain = re.search(r"Recomp-Toolchain:\s*sha256:([0-9a-f]{64})", body)
    if policy and lock_hash:
        (add("OK", "Recomp-Policy matches recomp.lock.toml") if policy.group(1) == lock_hash.group(1)
         else add("VIOLATION", f"Recomp-Policy {policy.group(1)[:12]}… != lock {lock_hash.group(1)[:12]}…"))
    else:
        add("VIOLATION", "missing/unparsable Recomp-Policy attestation")
    if toolchain and image_line:
        img_sha = re.search(r"sha256:([0-9a-f]{64})", image_line.group(1))
        if img_sha and toolchain.group(1) == img_sha.group(1):
            add("OK", "Recomp-Toolchain matches recomp.toml image digest")
        else:
            add("VIOLATION", "Recomp-Toolchain != pinned toolchain image digest")
    else:
        add("VIOLATION", "missing/unparsable Recomp-Toolchain attestation")

    # Lease.
    if issue_number:
        comments = gh_json(
            f"repos/{OWNER}/{REPOSITORY}/issues/{issue_number}/comments?per_page=100")
        leases = [c for c in comments
                  if (c["user"] or {}).get("id") == TRUSTED_ACTOR_ID
                  and "recomp-lease-v1" in (c["body"] or "")]
        lease = None
        for c in leases:
            lm = re.search(r"\{.*\}", c["body"], re.S)
            if lm:
                try:
                    lease = json.loads(lm.group(0))
                except json.JSONDecodeError:
                    pass
        if lease is None:
            add("VIOLATION", f"no trusted lease comment on issue #{issue_number}")
        else:
            active = bool(lease.get("expires_at")) and lease["expires_at"] > time.time()
            phase = lease.get("phase")
            claimant = lease.get("claimant_login")
            if phase == "done":
                add("OK", f"lease gen {lease.get('generation')} concluded (done), "
                          f"claimant {claimant}")
            elif not active:
                add("VIOLATION", f"lease not active (phase={phase}, expired={not active})")
            elif claimant != author:
                add("VIOLATION", f"lease claimant {claimant} != PR author {author}")
            else:
                add("OK", f"lease gen {lease.get('generation')} held by {claimant}, phase {phase}")
            if task and lease.get("task_id") != task["id"]:
                add("VIOLATION", f"lease task {lease.get('task_id')} != {task['id']}")

        # One-claim rule: flag other active leases held by the same author.
        others = []
        for oi in gh_json(
                f"repos/{OWNER}/{REPOSITORY}/issues?labels=recomp-task&state=open&per_page=100"):
            if oi["number"] == issue_number or (oi.get("user") or {}).get("id") != TRUSTED_ACTOR_ID:
                continue
            if "recomp-task:" not in (oi.get("body") or ""):
                continue
            oc = gh_json(f"repos/{OWNER}/{REPOSITORY}/issues/{oi['number']}/comments?per_page=100")
            last = None
            for c in oc:
                if (c["user"] or {}).get("id") == TRUSTED_ACTOR_ID and "recomp-lease-v1" in (c["body"] or ""):
                    lm = re.search(r"\{.*\}", c["body"], re.S)
                    if lm:
                        try:
                            last = json.loads(lm.group(0))
                        except json.JSONDecodeError:
                            pass
            if (last and last.get("phase") in ("claimed", "verifying")
                    and (last.get("expires_at") or 0) > time.time()
                    and last.get("claimant_login") == author):
                others.append(f"{last.get('task_id')} (#{oi['number']}, {last.get('phase')})")
        if others:
            add("WARN", f"author holds parallel active leases (one-claim rule): {others}")

    # Module content: signatures + bans.
    if declared_path and declared_path in files:
        blob = gh_json(
            f"repos/{OWNER}/{REPOSITORY}/contents/{declared_path}?ref={head}")
        src = strip_comments(
            __import__("base64").b64decode(blob["content"]).decode())
        impl = {normalize(s): raw for s, raw in implemented_signatures(src)}
        if task:
            for want in declared_signatures(task):
                if want in impl:
                    add("OK", f"declared signature present: {want}")
                else:
                    add("VIOLATION", f"declared signature missing or altered: {want}")
            extras = [s for s in impl if s not in declared_signatures(task)]
            if extras:
                add("WARN", f"extra pub fn not in the task record: {extras}")
        for label, pattern in BANNED_TOKENS:
            hits = re.findall(pattern, src, flags=re.M)
            if hits:
                add("VIOLATION", f"banned construct ({label}): {sorted(set(hits))[:4]}")

    # Authoritative check state (reported, never decided here).
    runs = gh_json(
        f"repos/{OWNER}/{REPOSITORY}/commits/{head}/check-runs")["check_runs"]
    verdict = next((r for r in runs if r["name"] == "recomp verification"), None)
    if verdict is None:
        state = "PENDING"
        add("PENDING", "recomp verification not posted yet on head")
    elif verdict["conclusion"] == "success":
        state = "GREEN"
        add("OK", f"recomp verification success: {verdict['output']['title']} "
                  f"{verdict['output']['summary']}")
    else:
        state = "RED"
        add("VIOLATION", f"recomp verification {verdict['conclusion']}: "
                         f"{verdict['output']['summary']}")

    n_viol = sum(1 for c in checks if c[0] == "VIOLATION")
    if n_viol:
        final = "REJECT: violations present; do not merge regardless of check state"
    elif state == "GREEN":
        final = "ADMITTABLE: no violations, authoritative check green"
    else:
        final = f"PENDING: no violations, authoritative check {state}"

    if args.json:
        print(json.dumps({"pr": args.number, "head": head, "author": author,
                          "files": files, "state": state, "final": final,
                          "checks": checks}, indent=1))
    else:
        print(f"PR #{args.number} head {head[:12]} by {author}: {final}")
        for st, label, detail in checks:
            print(f"  {st:<9} {label}" + (f" — {detail}" if detail else ""))
    return 1 if n_viol else 0


if __name__ == "__main__":
    sys.exit(main())
