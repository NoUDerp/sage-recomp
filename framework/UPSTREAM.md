# Vendored framework provenance

`src/recomp/` is the Recomp@Home framework this project's coordinator and
contributors run, vendored so the verdict code is version-controlled here
instead of drifting on a second repository's main branch.

- Base upstream: `pickles976/DecompAtHome` @ `8857811` (task start identity fix)
- Delta on top: branch `coordinator-fixes` @ `f6c7d05` —
  admissible contributor context (`SUMMARY.md`, `.mulch/`) beside the declared
  output; refusal codes published once per (pull request, head, reason);
  verified-but-unmerged merges retried per pass; `/release`; folded-base64 blob
  decode fix; fail-closed truncation flag; preflight action error detail;
  progress chart falls back to the newest parseable record; `framework_revision`
  in `health.json`; `coordinator.log`; one-claim guard in `task start`; submit
  admits context files.
- Re-vendor: apply the same delta to a newer upstream, run `framework/tests`,
  and update this file. The pin lives in `recomp.toml [toolchain].framework_rev`
  (a commit of *this* repository); `scripts/coordinator.sh serve` always runs
  this vendored tree, never an installed copy.
