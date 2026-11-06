# mv.py QMP W H X Y: only move the pointer, in small steps
import json,socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,**({"arguments":a} if a else {})})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
W,H,X,Y=map(int,sys.argv[2:6])
for i in range(21):
    y=Y-40+2*i
    cmd("input-send-event",{"events":[{"type":"abs","data":{"axis":"x","value":int(X*32767/W)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/H)}}]}); time.sleep(0.03)
