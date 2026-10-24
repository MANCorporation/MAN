# qmp.py sock click X Y | move X Y | key...  -- absolute pointer events via QMP (screen 1280x800)
import socket,sys,json,time
W,H=1280,800
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw')
f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,"arguments":a} if a else {"execute":c})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
def ev(*e): return cmd("input-send-event",{"events":list(e)})
def move(x,y): ev({"type":"abs","data":{"axis":"x","value":int(x*32767/W)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/H)}})
op=sys.argv[2]; x,y=float(sys.argv[3]),float(sys.argv[4])
move(x,y); time.sleep(0.15)
if op=="scroll":
    n=int(sys.argv[5]) if len(sys.argv)>5 else 3
    for _ in range(abs(n)):
        b="wheel-down" if n>0 else "wheel-up"
        ev({"type":"btn","data":{"down":True,"button":b}}); ev({"type":"btn","data":{"down":False,"button":b}}); time.sleep(0.05)
if op=="click":
    ev({"type":"btn","data":{"down":True,"button":"left"}}); time.sleep(0.08)
    ev({"type":"btn","data":{"down":False,"button":"left"}})
