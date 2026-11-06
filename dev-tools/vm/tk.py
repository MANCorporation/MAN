# tk.py MONITOR-SOCKET text: types on a US keyboard ("\n" = Enter)
import socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
SH={'-':'minus','.':'dot','/':'slash',' ':'spc',',':'comma','=':'equal',';':'semicolon'}
UP={'_':'shift-minus',':':'shift-semicolon','"':'shift-apostrophe'}
for c in sys.argv[2].encode().decode('unicode_escape'):
    k='ret' if c=='\n' else UP.get(c) or SH.get(c) or (f'shift-{c.lower()}' if c.isupper() else c)
    s.sendall(f"sendkey {k}\n".encode()); time.sleep(0.04)
time.sleep(0.2)
