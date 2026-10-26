# Contributing to sage-recomp

Humans and agents use the same Issue-driven workflow. One claim corresponds to
one Rust module and one pull request.

## Required environment

1. Use Linux or Windows through WSL2.
2. Install Git, Python 3.11+, and Docker or Podman.
3. Install the pinned framework:

   ```bash
   python3 -m pip install "git+https://github.com/pickles976/DecompAtHome.git@76627fdcbe52232d665e263825171c9b9612fc35"
   ```

4. Obtain your own legal copy of `game.dat` from BFME II patch 1.06:
   10,969,600 bytes, SHA-256
   `1aa5dca6595820e508df31352275e533cce93dd3be7b0ed7528530413ce99c5d`.
   Keep it outside the checkout.
5. Fork and clone this repository.
6. Run:

   ```bash
   recomp task doctor --project . --target /path/to/game.dat
   ```

Do not claim work until the final line is `GREENLIGHT`. Resolve every reported
error first. In particular, a hash or size mismatch means the game build is
wrong; do not bypass the check.

## Claiming work

```bash
recomp task next https://github.com/pickles976/sage-recomp
recomp task start <issue-url> --target /path/to/game.dat
```

`start` creates `recomp/<task-id>-<issue>`, posts `/claim`, and waits for the
coordinator App's trusted lease. A lease lasts two hours.
`recomp task heartbeat` renews it. If it expires, re-claim instead of submitting
unleased work.

## Implementing a batch

`recomp task run` writes the claimed task's initial
`recovery/crates/sage-core/src/recomp_tasks/port_NNNN.rs`. The Issue lists
9–12 x86 leaf functions and points to matching files under
`reference/decomp/`.

Implement every listed address in that one module. Keep the public `call`
signature and `Observed` contract described in [`PROMPT.md`](PROMPT.md).
Dependency additions, output renames, shared-module edits, generated expected
values, and target-derived bytes are outside task scope.

Run:

```bash
recomp task verify
```

The command builds the module in the digest-pinned container and compares all
declared cases against local Unicorn execution of your read-only target.
Passing locally is necessary, not authoritative.

## Submitting

```bash
recomp task submit
```

This re-verifies, pushes to your fork, and opens a pull request with one
`Closes #<issue>` reference and the required `Recomp-*` fields. Watch both
checks:

- `preflight`: trusted paths, policy version, task record, lease, artifact
  limits, and exact pull-request head.
- `recomp verification`: isolated candidate/oracle comparison by the owner's
  coordinator.

A stale policy requires rebasing and claiming under the new policy. A rejected
implementation returns the task to the pool after the configured failure
limit.

## Boundaries

Never commit or upload the original executable, game assets, extracted
resources, target paths, raw oracle logs, or build products. Never edit
`.github/`, `plugins/`, `toolchain/`, `recomp.toml`, `recomp.lock.toml`,
`recomp-launcher`, `docs/progress.*`, or `AGENTS.md` in a task pull request.
Report verifier defects in an Issue rather than working around them.
