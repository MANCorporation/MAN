#!/usr/bin/env python3
"""say.py OUTDIR 'cmd' ['cmd' ...] — send commands to live.py and wait until done."""
import os, sys, time
out = sys.argv[1]
ack = os.path.join(out, 'ack')
for c in sys.argv[2:]:
    # (Not by time stamp: file times are coarser than the clock.)
    try:
        os.unlink(ack)
    except FileNotFoundError:
        pass
    with open(os.path.join(out, 'cmd'), 'w') as f:
        f.write(c + '\n')
    while True:
        try:
            if open(ack).read().strip() == c.strip():
                break
        except FileNotFoundError:
            pass
        time.sleep(0.05)
