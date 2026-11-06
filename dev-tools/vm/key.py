# key.py MONITOR-SOCKET key...: QEMU sendkey names (esc, alt-f2, meta_l)
import socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
for k in sys.argv[2:]:
    s.sendall(f"sendkey {k}\n".encode()); time.sleep(0.5)
