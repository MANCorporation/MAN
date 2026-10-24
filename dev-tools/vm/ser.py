# ser.py sock "cmd" [wait]  -- send a line to serial console, print what comes back
import socket,sys,time
s=socket.socket(socket.AF_UNIX)
try: s.connect(sys.argv[1])
except OSError: sys.exit(3)
s.settimeout(0.5)
def drain(t):
    out=b''; end=time.time()+t
    while time.time()<end:
        try:
            b=s.recv(65536)
            if not b: break
            out+=b
        except socket.timeout: pass
    return out
drain(0.5)
s.sendall((sys.argv[2]+"\n").encode())
sys.stdout.write(drain(float(sys.argv[3]) if len(sys.argv)>3 else 3).decode(errors='replace'))
