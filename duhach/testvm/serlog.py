import socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.settimeout(1)
end=time.time()+float(sys.argv[3]); f=open(sys.argv[2],'wb')
while time.time()<end:
    try:
        d=s.recv(65536)
        if d: f.write(d); f.flush()
    except socket.timeout: pass
