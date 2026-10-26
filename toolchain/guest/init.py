#!/usr/bin/python3
"""Firecracker PID 1 wrapper.

Two things a container runtime provides and a raw Firecracker ext4 root does not:

* Device nodes. ``rustc`` fails outright without ``/dev/null`` (and LLVM reaches for
  ``/dev/urandom``), so PID 1 creates the standard set before the verifier runs.
* VMM shutdown. Linux stops at its power-off halt loop, but Firecracker only exits on
  a restart when the kernel is booted with ``reboot=k``; the wrapper rewrites that one
  final syscall so the host can read the result drive after the verdict is fsynced.
"""

from __future__ import annotations

import ctypes
import os
import stat
import sys

_RB_AUTOBOOT = 0x01234567
_real_cdll = ctypes.CDLL

# name -> (major, minor)
_DEVICE_NODES = {
    "null": (1, 3),
    "zero": (1, 5),
    "full": (1, 7),
    "random": (1, 8),
    "urandom": (1, 9),
    "tty": (5, 0),
}


def _create_device_nodes() -> None:
    for name, (major, minor) in _DEVICE_NODES.items():
        path = os.path.join("/dev", name)
        if os.path.exists(path):
            continue
        try:
            os.mknod(path, stat.S_IFCHR | 0o666, os.makedev(major, minor))
        except OSError as exc:
            # Serial console only. The verifier still reports the resulting failure as a
            # bounded diagnostic; nothing here turns it into a silent pass.
            print(f"guest init: cannot create {path}: {exc}", file=sys.stderr)


def _firecracker_libc(name, *args, **kwargs):
    library = _real_cdll(name, *args, **kwargs)
    reboot = library.reboot
    library.__dict__["reboot"] = lambda _command: reboot(_RB_AUTOBOOT)
    return library

# Firecracker execs PID 1 with an empty environment, and the wrapper script only sets
# PYTHONPATH. Without PATH, ``rustc`` cannot find the C linker it shells out to.
os.environ.setdefault("PATH", "/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin")
os.environ.setdefault("HOME", "/tmp")
os.environ.setdefault("TMPDIR", "/tmp")


_create_device_nodes()

# ``recomp.guest`` resolves libc while its module body runs, so the override has to be
# installed before that import.
ctypes.__dict__["CDLL"] = _firecracker_libc

from recomp.guest import main

raise SystemExit(main())
