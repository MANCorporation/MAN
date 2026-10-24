#!/usr/bin/env python3
"""type.py MONSOCK 'text' -- type text with HMP sendkey (US layout)."""
import socket, sys, time
special = {' ': 'spc', '\\': 'backslash', ':': 'shift-semicolon', '.': 'dot', '-': 'minus', '_': 'shift-minus',
           '/': 'slash', '!': 'shift-1', '\n': 'ret', '=': 'equal', ',': 'comma', ';': 'semicolon', '~': 'shift-grave_accent', '|': 'shift-backslash'}
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); time.sleep(0.2); s.recv(65536)
for ch in sys.argv[2]:
    if ch in special: k = special[ch]
    elif ch.isupper(): k = 'shift-' + ch.lower()
    else: k = ch
    s.sendall(f"sendkey {k}\n".encode()); time.sleep(0.06)
time.sleep(0.3)
