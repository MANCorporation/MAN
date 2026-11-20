#!/usr/bin/env python3
"""Chapter 1 take: power-on from the USB stick, the picker, the start-up screen,
the live session and the whole install (Erase disk, Prague, keyboard test).
The VM must be started paused: FILMQ=-S ./film-vm.sh F DISK ISO"""
import sys, time
from director import Scene
out = sys.argv[1]
with Scene('F.qmp', out) as s:
    s.q.cmd('cont')
    s.wait(7)                 # firmware, then the picker
    s.key('ret')              # Try or Install MAN
    s.wait(26)                # start-up screen, live desktop, Welcome
    s.move(959, 667, 1.0); s.wait(0.5); s.click(); s.wait(3)        # Install MAN...
    s.move(700, 600, 0.8); s.wait(0.6); s.move(959, 865, 0.8); s.click(); s.wait(3)   # disk page → Next
    s.move(1000, 342, 0.8); s.click(); s.wait(3.5)                   # Time Zone
    s.type('prague', cps=7); s.wait(1.5)
    s.move(959, 394, 0.7); s.click(); s.wait(1.5)
    s.move(900, 532, 0.8); s.click(); s.wait(0.4)
    s.type('Hello from MAN!', cps=10); s.wait(1.2)
    s.move(959, 865, 0.8); s.click(); s.wait(3)                     # → Ready to Install
    s.scroll(5, 0.12); s.wait(1); s.scroll(-5, 0.12); s.wait(1)
    s.move(959, 865, 0.8); s.click(); s.wait(1.5)                   # Erase and Install
    s.move(959, 593, 0.7); s.wait(0.4); s.click()                   # confirm
    s.wait(float(sys.argv[2]) if len(sys.argv) > 2 else 420)        # the install
