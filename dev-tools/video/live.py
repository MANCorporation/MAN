#!/usr/bin/env python3
"""live.py SOCK OUTDIR — keep filming while commands arrive one by one, so a
scene can be worked out step by step (the pauses are cut in editing).
Commands, one per line, sent with say.py: move X Y [DUR] | click | dclick | hold KEY MS | reset |
drag X Y [DUR] | key K [K..] | type TEXT | scroll N | wait S | mark NAME | stop
'mark' writes the current take time with a name to marks.txt (for editing)."""
import os, sys, time
from director import Scene

sock, out = sys.argv[1:3]
fifo = os.path.join(out, 'cmd')
os.makedirs(out, exist_ok=True)
if not os.path.exists(fifo):
    os.mkfifo(fifo)
marks = open(os.path.join(out, 'marks.txt'), 'a')
with Scene(sock, out) as s:
    s.q.cmd('cont')
    # Opened read-write: the pipe never reports end-of-file between commands
    # (a command arriving just as one writer closes would be lost).
    f = os.fdopen(os.open(fifo, os.O_RDWR), 'r', buffering=1)
    while True:
        line = f.readline()
        a = line.split()
        if not a:
            continue
        c = a[0]
        if c == 'move': s.move(float(a[1]), float(a[2]), float(a[3]) if len(a) > 3 else 0.7)
        elif c == 'click': s.click()
        elif c == 'dclick': s.click(double=True)
        elif c == 'drag': s.drag(float(a[1]), float(a[2]), float(a[3]) if len(a) > 3 else 1.0)
        elif c == 'key': s.key(*a[1:])
        elif c == 'hold':  # hold KEY MS: one key held down (e.g. Alt at power-on)
            s.q.cmd('send-key', {'keys': [{'type': 'qcode', 'data': a[1]}], 'hold-time': int(a[2])})
        elif c == 'reset': s.q.cmd('system_reset')
        elif c == 'type': s.type(line.split(' ', 1)[1].rstrip('\n'))
        elif c == 'scroll': s.scroll(int(a[1]), 0.1)
        elif c == 'wait': s.wait(float(a[1]))
        elif c == 'mark':
            marks.write(f"{time.time() - s.t0:.2f} {' '.join(a[1:])}\n"); marks.flush()
        s._mark('pos')
        tmp = os.path.join(out, 'ack.tmp')
        open(tmp, 'w').write(line)
        os.replace(tmp, os.path.join(out, 'ack'))
        if c == 'stop':
            break
os.unlink(fifo)
