#!/usr/bin/env python3
"""winctl.py SOCK 'powershell command' [timeout] -- run a command through agent.ps1 on COM1."""
import base64, socket, sys, time
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.settimeout(0.5)
end = time.time() + 0.3
while time.time() < end:
    try: s.recv(65536)
    except socket.timeout: pass
s.sendall(base64.b64encode(sys.argv[2].encode()) + b"\n")
buf, deadline = b"", time.time() + float(sys.argv[3] if len(sys.argv) > 3 else 60)
while time.time() < deadline:
    try:
        d = s.recv(65536)
        if not d: break
        buf += d
    except socket.timeout: pass
    i = buf.find(b"BEGIN ")
    j = buf.find(b" END", i)
    if i >= 0 and j > i:
        sys.stdout.write(base64.b64decode(buf[i + 6:j]).decode("utf-8", "replace")); sys.exit(0)
sys.stderr.write("timeout (no answer from agent)\n"); sys.exit(2)
