# Contributing to sage-recomp

Humans and agents use the same Issue-driven workflow. One claim corresponds to
one upstream C++ module, one Rust module, and one pull request. The work is a
port of `openbfme_core` (Open-BFME/openbfme-godot), which is GPL-3.0; your
contribution is GPL-3.0 too.

## Required environment

1. Use Linux or Windows through WSL2.
2. Install Git, the GitHub CLI (`gh`), Python 3.11+, and Docker or Podman.
3. Install the pinned framework. The pin is `framework_rev` under
   `[toolchain]` in `recomp.toml`:

   ```bash
   python3 -m pip install "git+https://github.com/pickles976/DecompAtHome.git@<framework_rev>"
   ```

4. Fork and clone this repository.
5. Run:

   ```bash
   recomp task doctor --project .
   ```

`./scripts/contributor.sh setup` performs steps 2, 3 and 5 on Ubuntu or Debian;
it prints what it will do and asks before using `sudo`. No copy of the game,
emulator, or C++ toolchain is needed: `cmake`, `ninja` and `g++` are
coordinator-only and are used solely to mint golden vectors.

Do not claim work until the final line is `GREENLIGHT`. Resolve every reported
error first; never bypass the check.

## Claiming work

Claims, pull requests and pushes are made with **your own** GitHub identity.
Either sign in once with `gh auth login`, or export a fine-grained personal
access token that can read and write Issues and pull requests on this
repository:

```bash
export GITHUB_TOKEN=<your fine-grained PAT>
```

The coordinator App's installation token does not work here: it cannot answer
who you are, and a claim must name a claimant. Never paste a token into an
Issue, a commit, or a log.
```bash
recomp task next https://github.com/pickles976/sage-recomp
recomp task start <issue-url>
```

`start` creates `recomp/<task-id>-<issue>`, posts `/claim`, and waits for the
coordinator App's trusted lease. A lease lasts two hours.
`recomp task heartbeat` renews it. If it expires, re-claim instead of submitting
unleased work.

## Implementing a task

`recomp task run` writes the claimed task's initial
`recovery/crates/sage-core/src/port/<module>.rs`. The Issue's task record names
the upstream module, the files to read at the pinned commit in
`reference/upstream.toml`, and every function to provide with its exact `pub`
Rust signature.

Implement all of them in that one module. It is compiled standalone by the
verifier, so it may use only `core` and `std`: no external crates, no
`use crate::…`, no `unsafe`, no I/O, no clock, no randomness, no global mutable
state. Results must be identical on every run and platform, floats included.
Dependency additions, output renames, shared-module edits, and hard-coded
expected values are outside task scope.

Run:

```bash
recomp task verify
```

The command regenerates 256 cases per function from the task's `case_seed`,
builds your module in the digest-pinned container, and compares the eight
case-group digests with `goldens/<task-id>.json`. Passing locally is necessary,
not authoritative.

## Submitting

```bash
recomp task submit
```

This re-verifies, pushes to your fork, and opens a pull request with one
`Closes #<issue>` reference and the required `Recomp-*` fields. Watch both
checks:

- `preflight`: trusted paths, policy version, task record, lease, artifact
  limits, and exact pull-request head.
- `recomp verification`: isolated golden comparison by the owner's coordinator
  in a networkless guest.

A stale policy requires rebasing and claiming under the new policy. A rejected
implementation returns the task to the pool after the configured failure limit.

## Boundaries

Never commit `.cache/`, any file larger than 256 KB, binaries, wheels, assets,
build output, or Git LFS objects. Never edit `.github/`, `plugins/`,
`toolchain/`, `goldens/`, `reference/`, `tools/`, `scripts/`, `recomp.toml`,
`recomp.lock.toml`, `recomp-launcher`, `docs/progress.*`, or `AGENTS.md` in a
task pull request. Report verifier defects in an Issue rather than working
around them.
