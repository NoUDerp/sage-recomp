#!/usr/bin/python3
"""Firecracker PID 1 wrapper.

Linux reaches its power-off halt loop, but Firecracker only exits the VMM on a
restart when the kernel is booted with ``reboot=k``. This wrapper rewrites that one
final shutdown command so the trusted guest can fsync its verdict and the host can
then read the result drive.
"""

from __future__ import annotations

import ctypes

_RB_AUTOBOOT = 0x01234567
_real_cdll = ctypes.CDLL


def _firecracker_libc(name, *args, **kwargs):
    library = _real_cdll(name, *args, **kwargs)
    reboot = library.reboot
    library.__dict__["reboot"] = lambda _command: reboot(_RB_AUTOBOOT)
    return library


# ``recomp.guest`` resolves libc while its module body runs, so the override has to
# be installed before that import.
ctypes.__dict__["CDLL"] = _firecracker_libc

from recomp.guest import main  # noqa: E402  (import order is load-bearing)

raise SystemExit(main())
