# AGENTS.md — sage-recomp

This file governs human and AI contributors. Read it and `PROMPT.md` before
claiming work.

## Project facts

- Target: The Lord of the Rings: The Battle for Middle-earth II, patch 1.06,
  32-bit `game.dat`; identity is pinned in `recomp.toml`.
- Scope: executable recovery only. This repository must never contain the
  executable, game assets, extracted resources, or target-derived blobs.
- Goal: safe, deterministic Rust for a Bevy 0.19 engine port.
- Plugin: `lotr-bevy` in `plugins/lotr-bevy/`; tier `t2-recomp`.
- Issues are authoritative. Labels are presentation only. Trusted task records
  and lease comments from `sage-recomp[bot]` define admissible work.

## Mandatory readiness gate

Run this before `task next` or `task start`:

```bash
recomp task doctor --project . --target /path/to/game.dat
```

Proceed only after `GREENLIGHT`. A target failure requires the patch 1.06
`game.dat`, exactly 10,969,600 bytes with SHA-256
`1aa5dca6595820e508df31352275e533cce93dd3be7b0ed7528530413ce99c5d`.
Tell the human that exact requirement; never weaken or bypass the check.

## Workflow

```bash
recomp task next https://github.com/pickles976/sage-recomp
recomp task start <issue-url> --target /path/to/game.dat
recomp task run
# implement only the declared port_NNNN.rs
recomp task verify
recomp task submit
```

- One current claim, one declared output module, one pull request, one
  `Closes #<issue>` reference.
- Claims last two hours. Renew with `recomp task heartbeat`; re-claim after an
  expiry.
- Implement every function listed in the Issue. Read the matching
  `reference/decomp/<address>.c`; preserve the generated `call` and `Observed`
  ABI.
- Keep code safe, deterministic, dependency-free, and free of target bytes or
  case-specific expected values.
- Only the coordinator's isolated verification and exact-head merge count as
  progress. Local success is not authority.

## Maintainer-owned paths

Never edit `.github/`, `plugins/`, `toolchain/`, `recomp.toml`,
`recomp.lock.toml`, `recomp-launcher`, `docs/progress.*`, or this file in a task
pull request. Preflight rejects those changes. Do not edit any recovery file
outside the Issue's declared output path.

## Verification boundary

Candidate Rust runs in a disposable, networkless Firecracker guest without the
target. The x86 oracle runs separately with the owner's read-only target and
without contributor source. The coordinator compares only stable case digests.
Never attempt to expose host paths, target bytes, raw logs, or guest internals.
Report a verifier defect in an Issue instead of bypassing it.

## Final report

When stopping, list tasks claimed/merged/rejected, the policy version, your
real model name, minutes per task, and abandoned tasks with reasons.
