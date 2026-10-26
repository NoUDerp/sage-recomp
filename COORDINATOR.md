# COORDINATOR.md — owner-side coordinator session brief

Operational notes for a session acting as the **coordinator** of
`pickles976/sage-recomp`: watch for inbound pull requests, confirm the
verifier's verdict, merge or leave them. `AGENTS.md` still governs contributor
work and repo rules; this file only carries the state and procedures a fresh
coordinator session needs. Facts below were verified on 2026-10-26.

## Who is authoritative

- The verdict comes from the polling loop: `scripts/coordinator.sh serve` →
  `recomp coordinator run --owner pickles976 --repository sage-recomp`, launched
  under `sudo setpriv` (jailer caps `+sys_admin,+sys_chroot,+setuid,+setgid,
  +net_admin,+mknod`). It polls GitHub every 60 s (`--poll-interval`), runs the
  candidate module in a disposable networkless Firecracker guest, compares the 8
  case-group digests with `goldens/` from the trusted base tree, posts the
  `recomp verification` check run, and merges the exact head on a pass.
- A coordinator session never re-verifies by hand and never merges by eye.
  `recomp task verify` in a contributor tree is a convenience, not a verdict.
  Goldens and case regeneration live only in the guest.

## First three commands of a session

```bash
pgrep -af 'coordinator run' || echo 'NO LOOP'
cat ~/.local/state/recomp/coordinators/pickles976__sage-recomp/health.json
gh pr list --repo pickles976/sage-recomp --state open \
  --json number,title,headRefName,createdAt
```

`health.json` (schema 2): `admitted_total`, `merged_total`, `pending`,
`verified_unmerged`, `progress_failures`, `last_reconcile_at` (unix seconds). A
stale `last_reconcile_at` means the loop died. Same directory holds
`candidates.db` (claim/candidate state), `quarantine/<task-id>-…` (rejected
attempts; `port-0005` has two) and `recomp-coordinator.service`. The loop's
stdout goes to its own pty and is not readable from an unprivileged session —
use `health.json` plus the GitHub check run instead of hunting for logs.

## Deciding on an open pull request

Read the check run on the **PR head**, not on the squash commit (squashing
creates a new commit with no check runs):

```bash
H=$(gh pr view <n> --repo pickles976/sage-recomp --json headRefOid -q .headRefOid)
gh api repos/pickles976/sage-recomp/commits/$H/check-runs \
  -q '.check_runs[]|{name,conclusion,app:.app.slug,title:.output.title,summary:.output.summary}'
```

- `recomp verification` (app `sage-recomp`) conclusion `success`, title
  `verification passed`, summary `{"codes":[],"evidence":{"outputs":8,"verifier_mode":"static"}}`
  → the head is admissible. The loop merges it; if it has not, recover below.
- Green check but the pull request is still open and `health.json` shows
  `verified_unmerged: 1` → squash-merge exactly that sha, per
  `README.md` § *If a verified pull request is left unmerged* (`PUT
  /repos/.../pulls/<n>/merge` with `"merge_method":"squash"`). Never merge a
  head whose check run is not green.
- `recomp verification` failure → the summary carries stable reason `codes` and
  counts, never host paths or raw logs. The Issue's lease/verdict comment is the
  record; leave the module alone. Do not patch a contributor's branch, and do not
  widen the case set to make a failure go away.
- `preflight` (check-run app `github-actions`, the
  `pickles976/DecompAtHome/.github/actions/recomp-preflight@936de74` action in
  `.github/workflows/recomp-preflight.yml`) is advisory: pull request #21 merged
  with `preflight` FAILURE and `recomp verification` SUCCESS. Consult it only to
  explain an anomaly, never as the merge gate.

Contributor pull requests may change only their Issue's declared
`recovery/crates/sage-core/src/port/<module>.rs` with exactly the declared `pub`
signatures; the loop's preflight rejects policy-owned paths. Never merge a head
that edited `goldens/`, `plugins/`, `toolchain/`, `reference/`, `tools/`,
`scripts/`, `.github/`, `docs/progress.*`, `recomp.toml`, `recomp.lock.toml`,
`recomp-launcher` or `AGENTS.md`.

Before merging, cross-check the pull request body's own attestations
(`gh pr view <n> --json body`): one `Closes #<issue>` for the leased task, plus
`Recomp-Policy: sha256:<policy hash>`, `Recomp-Toolchain: sha256:<…>`,
`Recomp-Image: ghcr.io/…@sha256:<…>`, `Recomp-Lease: <generation>` and
`Recomp-Task-Generation: <n>`. On pull request #21 the policy sha equalled
`recomp.lock.toml`'s `hash` and the toolchain/image sha equalled
`recomp.toml [toolchain]`. If either differs, the contributor verified under a
different policy than this tree declares: say so on the pull request instead of
merging on the green check alone.

A pull request is expected to carry a `SUMMARY.md` with the contributor's own
context about the port (ambiguities met, decisions taken). Read it during
review and carry what matters into the merge record; absence is not a
rejection reason, and the golden verdict outranks anything written in it. If
the loop ever rejects a pull request *solely* for carrying `SUMMARY.md`, that
is a verifier defect: report it in an Issue, do not bypass the gate.

After merging, fold the summary into `docs/HISTORY.md` (append a dated section
with the PR number, task, verdict line, and the summary body) and `git rm` the
root `SUMMARY.md` the merge brought in — the next PR would otherwise overwrite
it. `docs/HISTORY.md` is coordinator-written only: a contributor pull request
that modifies it is a policy violation, reject it.

## Merge record comment (mandatory)

Every merged pull request gets a coordinator comment as it merges — the trail
must not depend on re-reading check runs afterwards. Post it with
`gh pr comment <n> --body-file` and keep these fields, all grounded:

1. **Verdict** — `recomp verification` conclusion, title, and the exact
   `evidence`/`codes` summary from the head, plus which golden and `case_seed`
   the guest compared against.
2. **Lease** — Issue number, `recomp-lease-v1` claimant and generation, active
   at verification time.
3. **Attestations** — `Recomp-Policy` vs `recomp.lock.toml`, toolchain/image vs
   `recomp.toml`; that the diff touches only the declared output module and
   implements the declared functions.
4. **Preflight** — state of the advisory check, and why it does not gate here.
5. **Merge** — exact verified head sha → squash sha, and who merged (loop pass
   or coordinator recovery, with the reason if recovered).
6. **`SUMMARY.md`** — whether one accompanied the PR; if yes, the points from
   it that affected the decision.

Backfilled example: the 2026-10-26 comments on pull requests #21 and #22.

## Progress chart during each pull request

`docs/progress.json` + `docs/progress.svg` (the README badge) are
maintainer-owned and must move with the lane. The loop republishes them after
its *own* merges (`_publish_progress_best_effort` in the framework), but a
coordinator recovery merge bypasses that, and the publish is best-effort
silent. So: after every merge — loop or recovery — run

```bash
python3 tools/refresh_progress.py --commit
```

It reuses the framework's own `recomp.progress` collector over `gh api`, so the
output is exactly what the loop would publish; never hand-edit the JSON or SVG.
Known collector quirk (2026-10-26): Issues #9, #10 and #12 carry superseded
task-record comments and `parse_trusted_task_record` rejects them, so the chart
counts 14 of 16 tasks and skips `port-0005` even though it is done. That is
framework-faithful output, not a chart bug; report it upstream instead of
patching the rendered files.

## Restarting the loop needs a human

Verified on this host:

- `sebas` is not in group `kvm`; `/dev/kvm` is `crw-rw---- root kvm`, so
  `recomp coordinator doctor` reports `error kvm | /dev/kvm is not read/write
  for this account` for an unprivileged account.
- `sudo -n true` fails: the `setpriv` line needs an interactive password. Only
  `/usr/bin/systemctl` is NOPASSWD for `sebas`.

So a session cannot bring the verifier up by itself. Ask the human to run it in
a named tmux session (`tmux new -s sage-coordinator`, one sudo prompt, then
detach with `Ctrl-b d`) and run `./scripts/coordinator.sh serve`. `recomp
coordinator run --max-passes 1` is the single-pass smoke form.

## Pins and drift to check once per session

- `policy_version = 18` and the file hashes in `recomp.lock.toml`. Editing any
  locked file (`recomp.toml`, `recomp-launcher`, `plugins/sage-port/*`,
  `toolchain/*`) requires re-locking plus `scripts/coordinator.sh check`.
- Three framework revisions are live at once and they differ:
  `recomp.toml [toolchain] framework_rev = 54ee7e1f…` (what contributors
  `pip install`, and what `README.md` pins), `scripts/coordinator.sh`
  `FRAMEWORK_PIN = 76627fdc…`, and the running loop, which executes the checkout
  at `~/Projects/DecompAtHome/.venv` (`git -C ~/Projects/DecompAtHome
  rev-parse HEAD` was `88578113…` on 2026-10-26). Drift means the verdict the
  repo publishes is not the one the running loop produced: report it, don't
  paper over it.
- Toolchain image digest `sha256:32a34a3e…` in `recomp.toml`; `recomp task
  doctor` and `recomp coordinator doctor` both confirm it is present.

## Task lane state (2026-10-26)

- 16 tasks `port-0001`…`port-0016` in `plugins/sage-port/task-index.json`
  (`weight`, `case_seed`, and the exact `pub` API per task);
  `goldens/port-XXXX.json` exists for every one, so minting is not needed for
  normal merging. Each golden: `case_groups: 8` × `cases_per_group: 32` = 256
  cases, one digest per group (`case-00`…`case-07`, matching
  `recomp.plugin.toml [verify.protocol]`), plus 8–32 visible `examples` rows
  that vary with the declared API. Both sides regenerate cases from the task's
  `case_seed`.
- Current task lane: Issues #6…#20 = `port-0002`…`port-0016`, titled
  `port-XXXX: port-module`. Issues #1, #2, #4, #5 are superseded
  `t2-rust-differential` drafts from the earliest policy versions — ignore them.
  Labels are presentation only; `recomp-task`, `open`, `claimed` come from the
  loop. Issue #20 = `port-0016`: `pickles976` posted `/claim` at 17:55 and
  `sage-recomp[bot]` answered with the trusted
  `<!-- recomp-lease-v1 … -->` comment (`phase: claimed`, `claimant_login`,
  `expires_at` ≈ +2 h, `generation`, `verdict`). Only that bot's comments are
  admissible (`trusted_actor_id = 348113819`), and GitHub rejects the App token
  as a claim credential, so claims must come from a real account.
- Merged through the loop so far: `port-0005` (Issue #9, pull request #21,
  squash commit `2138b09`). The other files under
  `recovery/crates/sage-core/src/port/` (16 modules plus `mod.rs`) are
  owner-seeded stubs and pre-loop ports, not loop progress. `docs/progress.json`
  read empty (`generated_at: 0`, all totals 0) on 2026-10-26 while a task was
  leased; the loop regenerates it, so that lag is cosmetic, not progress.
- Upstream specification is the checkout pinned in `recomp.toml`
  (`Open-BFME/openbfme-godot` @ `f45ea04a…`). Judge a module only against those
  files at that commit.

## Owner-only work a session may do

`./scripts/coordinator.sh` subcommands: `doctor` (owner prerequisites + the
project gate), `check` (re-mint-check all goldens byte-for-byte, plugin
selftest, `cargo test` in `recovery/`), `reference` (clone + build the pinned
upstream core into `.cache/`), `mint [task…]` (owner-only golden minting via
`tools/mint_goldens.py`), `image`/`rootfs` (digest-pinned toolchain and
Firecracker rootfs), `publish` (`recomp github sync-tasks`), `serve`. Plus owner
commits on `main` to policy-owned files. Never hand-edit a golden; mint or
re-mint it. `.cache/` stays uncommitted, no committed file over 256 KB, no
binaries or LFS objects.

`recomp task next/start/heartbeat/submit` post as the *contributor* account via
`gh auth login` or `GITHUB_TOKEN`; the coordinator App token is rejected for
claims. A coordinator session should not be claiming tasks unless it is also
running the contributor lane.

## Cost discipline

The loop polls every 60 s by itself, so a coordinator session should be idle
most of the time: check for an open pull request on a long interval, act only
when a head has a check run, and never busy-poll `health.json` or the guest.

## End-of-session report

Per `AGENTS.md`: tasks claimed/merged/rejected, the policy version, the real
model name, minutes per task, and any abandoned task with its reason.
