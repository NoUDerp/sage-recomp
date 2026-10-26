# AGENTS.md — sage-recomp

This file governs human and AI contributors. Read it and `PROMPT.md` before
claiming work.

## Project facts

- Goal: port `openbfme_core`, the Godot-free SAGE engine core of
  Open-BFME/openbfme-godot, from C++ to safe deterministic Rust.
- Upstream repository and commit are pinned in `reference/upstream.toml`.
  Upstream is GPL-3.0, so this port is GPL-3.0.
- Scope: source translation only. This repository contains no game executable,
  assets, or extracted resources, and never needs them.
- Plugin: `sage-port` in `plugins/sage-port/`; verification is static golden
  comparison, one guest, no target binary.
- Issues are authoritative. Labels are presentation only. Trusted task records
  and lease comments from `sage-recomp[bot]` define admissible work.

## Coordinator sessions

A session acting as the coordinator — polling the remote, verifying and merging
inbound pull requests — reads [`COORDINATOR.md`](COORDINATOR.md) first. It
records where authority actually lives, how to tell whether the verifier loop is
alive, the merge gate, the comment every merge must leave, and the host
prerequisites that need the owner's password. Everything else in this file still
applies.

## Mandatory readiness gate

Run this before `task next` or `task start`:

```bash
recomp task doctor --project .
```

Proceed only after `GREENLIGHT`. This project declares no target binary; never
pass `--target` and never weaken or bypass the check.

## Workflow

```bash
recomp task next https://github.com/pickles976/sage-recomp
recomp task start <issue-url>
recomp task run
# implement only the declared recovery/crates/sage-core/src/port/<module>.rs
recomp task verify
recomp task submit
```

- One current claim, one declared output module, one pull request, one
  `Closes #<issue>` reference.
- Claims last two hours. Renew with `recomp task heartbeat`; re-claim after an
  expiry.
- Implement every API entry listed in the Issue with exactly the declared `pub`
  signature. The upstream C++ files named in the task record are the
  specification; read them at the pinned commit.
- The module is compiled standalone: `core` and `std` only, no external crates,
  no `use crate::…`, no `unsafe`, no I/O, no clock, no randomness, no global
  mutable state. Floats must reproduce the reference bit pattern.
- Never hard-code case-specific expected values or golden digests.
- Only the coordinator's isolated verification and exact-head merge count as
  progress. Local success is not authority.

## Maintainer-owned paths

Never edit `.github/`, `plugins/`, `toolchain/`, `goldens/`, `reference/`,
`tools/`, `scripts/`, `recomp.toml`, `recomp.lock.toml`, `recomp-launcher`,
`docs/progress.*`, or this file in a task pull request. Preflight rejects those
changes. Do not edit any recovery file outside the Issue's declared output path.

## Repository rules

Never commit `.cache/`, never commit a file larger than 256 KB, and never add
binaries, wheels, assets, build output, or Git LFS objects.

## Verification boundary

Candidate Rust runs in a disposable, networkless Firecracker guest with no
target drive. The guest regenerates the cases from the task's `case_seed`,
compiles the candidate module alone, and compares its case-group digests with
the committed `goldens/<task-id>.json`, which always comes from the trusted base
tree. Never attempt to expose host paths, golden contents, raw logs, or guest
internals. Report a verifier defect in an Issue instead of bypassing it.

## Final report

When stopping, list tasks claimed/merged/rejected, the policy version, your
real model name, minutes per task, and abandoned tasks with reasons.
