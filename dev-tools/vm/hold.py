# hold.py MONITOR-SOCKET KEY SECONDS: keeps KEY down (re-pressed every 0.5 s,
# like a real keyboard's auto-repeat) for SECONDS.
import socket, sys, time
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
end = time.time() + float(sys.argv[3])
while time.time() < end:
    s.sendall(f"sendkey {sys.argv[2]} 60\n".encode()); time.sleep(0.1)
