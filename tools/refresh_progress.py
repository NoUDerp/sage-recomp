#!/usr/bin/env python3
"""Regenerate docs/progress.json and docs/progress.svg (owner/coordinator only).

Derives task state from trusted Issue records exactly like the verification
loop does, by reusing recomp.progress on top of a `gh api` adapter, so the
committed chart is identical to what the loop would publish. The loop republishes
after its own merges, but a coordinator recovery merge bypasses it, so run this
after every merge and commit the result:

    python3 tools/refresh_progress.py [--commit]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Iterator

from recomp.progress import collect_progress, render_progress_json, render_progress_svg

OWNER = "pickles976"
REPOSITORY = "sage-recomp"
TRUSTED_ACTOR_ID = 348113819  # sage-recomp[bot], from recomp.toml [github]
PROJECT = "sage-recomp"
ROOT = Path(__file__).resolve().parent.parent


def gh_paginate(path: str) -> list[Any]:
    out = subprocess.run(
        ["gh", "api", "--paginate", path, "--slurp"],
        capture_output=True, text=True, check=True).stdout
    # --slurp wraps every page in its own array element.
    items: list[Any] = []
    for page in json.loads(out):
        items.extend(page)
    return items


class GhAdapter:
    """The two recomp.progress entry points, backed by the gh CLI."""

    def list_issues(self) -> Iterator[Any]:
        return iter(gh_paginate(f"repos/{OWNER}/{REPOSITORY}/issues?state=all"))

    def list_comments(self, issue_number: int) -> Iterator[Any]:
        return iter(gh_paginate(f"repos/{OWNER}/{REPOSITORY}/issues/{issue_number}/comments"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", action="store_true",
                        help="commit docs/progress.* to main when they change")
    args = parser.parse_args()

    snapshot = collect_progress(GhAdapter(), project=PROJECT,
                                trusted_actor_id=TRUSTED_ACTOR_ID,
                                now=int(time.time()))
    writes = {
        ROOT / "docs/progress.json": render_progress_json(snapshot),
        ROOT / "docs/progress.svg": render_progress_svg(snapshot),
    }
    for path, content in writes.items():
        if path.read_text() != content:
            path.write_text(content)
            print(f"updated {path.relative_to(ROOT)}")
        else:
            print(f"unchanged {path.relative_to(ROOT)}")

    totals = snapshot.totals()
    print("totals:", json.dumps(totals, sort_keys=True))
    print("tasks:", " ".join(f"{t.task_id}:{t.status}" for t in snapshot.sorted_tasks()))

    if not args.commit:
        return 0
    changed = [str(p.relative_to(ROOT)) for p in writes if p.exists()
               and subprocess.run(["git", "diff", "--quiet", "--", str(p.relative_to(ROOT))],
                                  cwd=ROOT).returncode != 0]
    if not changed:
        print("nothing to commit")
        return 0
    subprocess.run(["git", "add", *changed], cwd=ROOT, check=True)
    subprocess.run(["git", "commit", "-m",
                    "coordinator: refresh progress chart"], cwd=ROOT, check=True)
    subprocess.run(["git", "push", "origin", "HEAD:main"], cwd=ROOT, check=True)
    print("pushed:", ", ".join(changed))
    return 0


if __name__ == "__main__":
    sys.exit(main())
