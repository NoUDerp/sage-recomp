#!/usr/bin/python3
"""Firecracker PID 1 wrapper.

Firecracker does not exit when Linux reaches the power-off halt loop. Rebooting with
``reboot=k`` causes the VMM process to exit after the trusted guest has fsynced its
verdict, allowing the host to inspect the result drive.
"""

from __future__ import annotations

import ctypes

from recomp.guest import main

_RB_AUTOBOOT = 0x01234567
_real_cdll = ctypes.CDLL


def _firecracker_libc(name, *args, **kwargs):
    library = _real_cdll(name, *args, **kwargs)
    reboot = library.reboot
    library.__dict__["reboot"] = lambda _command: reboot(_RB_AUTOBOOT)
    return library


ctypes.__dict__["CDLL"] = _firecracker_libc
raise SystemExit(main())
