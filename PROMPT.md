# sage-recomp contributor prompt

You are porting the SAGE engine core to safe Rust. The reference is
`openbfme_core`, the Godot-free C++ core of
https://github.com/Open-BFME/openbfme-godot (GPL-3.0). Work is coordinated
through https://github.com/pickles976/sage-recomp under policy v7. Only merged pull
requests count.

## Readiness gate

Before claiming work, run:

```bash
recomp task doctor --project .
```

Stop unless the last line is `GREENLIGHT`. This project pins no game binary and
needs no game files: everything a contributor needs is in the repository and the
pinned toolchain image.

## Claim, port, publish

```bash
recomp task next https://github.com/pickles976/sage-recomp
recomp task start <issue-url>
recomp task run
# implement only the declared recovery/crates/sage-core/src/port/<module>.rs
recomp task verify
recomp task submit
```

`start` waits for the coordinator App's trusted two-hour lease. Use
`recomp task heartbeat` to renew it. One claim, one output module, one pull
request, and exactly one `Closes #<issue>` reference.

Each Issue names one upstream C++ module and the exact Rust API your port must
expose. For that module:

1. Read the upstream sources listed in the Issue at the pinned commit, e.g.
   https://github.com/Open-BFME/openbfme-godot/blob/<commit>/engine/src/Common/MD5.cpp
   Do not copy build output, generated files or anything from outside those files.
2. Implement every declared function with the declared signature in the declared
   file. The module is compiled **on its own** by the verifier, so it must not
   reference other modules of the crate: `core` and `std` only, no external
   crates, no `unsafe`, no I/O, no clock, no randomness, no global mutable state.
3. Run `recomp task verify`. It generates 256 cases per function from the task's
   seed, runs your module, and compares eight case-group digests against
   `goldens/<task-id>.json`, which the project owner minted from the C++
   reference build. The coordinator repeats the same verification inside a
   networkless Firecracker guest and merges only an exact head.
4. `goldens/`, `reference/`, `tools/`, `scripts/`, `plugins/`, `toolchain/` and
   the policy files are maintainer-owned; a pull request that touches them is
   rejected by preflight.

Do not hard-code expected outputs, do not special-case the visible example rows
in the golden file, and do not add dependencies, build output, local paths or
files over 256 KB. If the upstream behaviour is ambiguous (platform-dependent
float formatting, undefined order, uninitialised reads), report it in the pull
request instead of guessing.

The port is GPL-3.0 because the reference is.

## Final report

When stopping, report tasks claimed/merged/rejected, policy
v7, your real model name, minutes per task, and abandoned tasks
with reasons.
