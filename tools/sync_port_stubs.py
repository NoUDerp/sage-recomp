#!/usr/bin/env python3
"""Keep `recovery/crates/sage-core/src/port/` in step with the task index.

Owner-only. Every open task gets a committed stub so the workspace always
compiles; a task that a contributor has already landed is left untouched.

    tools/sync_port_stubs.py            # write missing stubs, rewrite mod.rs
    tools/sync_port_stubs.py --check    # fail if anything is out of date
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT / "plugins" / "sage-port"))

import runner  # noqa: E402  (path set above)

PORT_DIR = PROJECT_ROOT / "recovery" / "crates" / "sage-core" / "src" / "port"
HEADER = """//! Rust ports of openbfme_core modules, one module per crowdsourced task.
//!
//! Each file is a standalone, safe, deterministic translation of one upstream
//! C++ module; the verifier compiles it on its own against the golden vectors
//! in `goldens/`. Files still carrying `unimplemented!()` are open tasks.

"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    options = parser.parse_args()
    tasks = sorted(runner.task_entries(), key=lambda task: task["id"])
    PORT_DIR.mkdir(parents=True, exist_ok=True)
    stale: list[str] = []
    for task in tasks:
        path = PORT_DIR / f"{task['rust_mod']}.rs"
        if path.exists():
            continue
        if options.check:
            stale.append(f"missing stub {path.relative_to(PROJECT_ROOT)}")
            continue
        path.write_text(runner.stub_source(task), encoding="utf-8")
        print(f"wrote {path.relative_to(PROJECT_ROOT)}")
    modules = sorted({task["rust_mod"] for task in tasks})
    listing = HEADER + "".join(f"pub mod {name};\n" for name in modules)
    mod_path = PORT_DIR / "mod.rs"
    current = mod_path.read_text(encoding="utf-8") if mod_path.exists() else ""
    if current != listing:
        if options.check:
            stale.append("port/mod.rs does not list exactly the indexed tasks")
        else:
            mod_path.write_text(listing, encoding="utf-8")
            print(f"wrote {mod_path.relative_to(PROJECT_ROOT)}")
    for message in stale:
        print(f"stale: {message}")
    return 1 if stale else 0


if __name__ == "__main__":
    raise SystemExit(main())
