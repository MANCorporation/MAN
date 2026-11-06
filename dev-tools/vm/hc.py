# hc.py QMP X Y [W H]: human-like click: glide, pause, hold the button 120 ms
import json,socket,sys,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,**({"arguments":a} if a else {})})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
X,Y=int(sys.argv[2]),int(sys.argv[3]); W,H=(int(sys.argv[4]),int(sys.argv[5])) if len(sys.argv)>5 else (1280,800)
def mv(x,y): cmd("input-send-event",{"events":[{"type":"abs","data":{"axis":"x","value":int(x*32767/W)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/H)}}]})
for i in range(21): mv(X-60+3*i,Y-40+2*i); time.sleep(0.02)
time.sleep(0.5)
cmd("input-send-event",{"events":[{"type":"btn","data":{"down":True,"button":"left"}}]}); time.sleep(0.12)
cmd("input-send-event",{"events":[{"type":"btn","data":{"down":False,"button":"left"}}]}); time.sleep(0.3)
