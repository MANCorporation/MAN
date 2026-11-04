#!/usr/bin/env python3
"""vmrun.py SOCK TIMEOUT 'command'  — run a shell command on the serial console
of a VM (logged in as root) and print its output; exit with its exit code.
Output is wrapped in markers so leftovers on the console are ignored."""
import os, re, socket, sys, time
sock, timeout, cmd = sys.argv[1], float(sys.argv[2]), sys.argv[3]
tag = f"M{os.getpid()}{int(time.time())}"
s = socket.socket(socket.AF_UNIX)
s.connect(sock)
s.settimeout(0.5)
def read(t):
    out, end = b"", time.time() + t
    while time.time() < end:
        try:
            b = s.recv(65536)
            if not b:
                break
            out += b
        except socket.timeout:
            pass
    return out
read(0.3)
# stty -echo: the console does not repeat the command back
s.sendall(f"stty -echo cols 250; echo {tag}A; {{ {cmd}\n}} 2>&1; echo {tag}Z$?\n".encode())
buf, end = b"", time.time() + timeout
while time.time() < end:
    buf += read(1)
    m = re.search(rb"%sA\r?\n(.*)%sZ(\d+)" % (tag.encode(), tag.encode()), buf, re.S)
    if m:
        sys.stdout.write(m.group(1).decode(errors="replace").replace("\r", ""))
        sys.exit(int(m.group(2)))
sys.stdout.write(buf.decode(errors="replace")[-3000:])
print(f"\n[vmrun: timeout after {timeout:.0f} s]")
sys.exit(124)
