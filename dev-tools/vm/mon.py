import socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); time.sleep(0.3); s.recv(65536)
s.sendall((" ".join(sys.argv[2:])+"\n").encode()); time.sleep(1.0)
try:
    s.settimeout(1); s.recv(65536)
except Exception: pass
