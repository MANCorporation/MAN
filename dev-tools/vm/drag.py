# drag.py QMP W H X1 Y1 X2 Y2: press, move, keep holding; drag.py QMP W H up: release
import json,socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,**({"arguments":a} if a else {})})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
W,H=int(sys.argv[2]),int(sys.argv[3])
def mv(x,y): cmd("input-send-event",{"events":[{"type":"abs","data":{"axis":"x","value":int(x*32767/W)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/H)}}]})
def btn(d): cmd("input-send-event",{"events":[{"type":"btn","data":{"down":d,"button":"left"}}]})
if sys.argv[4]=='up': btn(False); sys.exit()
x1,y1,x2,y2=map(int,sys.argv[4:8])
mv(x1,y1); time.sleep(0.4); btn(True); time.sleep(0.2)
for i in range(1,41): mv(x1+(x2-x1)*i/40,y1+(y2-y1)*i/40); time.sleep(0.03)
time.sleep(1.2)
