# armwait.py SOCK SECONDS: read the serial port without typing, until the root shell prompt shows
import socket,sys,time,re
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.settimeout(1); end=time.time()+float(sys.argv[2]); b=b''
while time.time()<end:
    try: b=(b+s.recv(65536))[-4000:]
    except socket.timeout: pass
    if re.search(rb'(root@[^\n]*[#] ?$|login: ?$)',b.rstrip(b' ')+b' ') or re.search(rb'# $',b): print('PROMPT',b[-80:]); sys.exit(0)
print('NONE',b[-300:])
