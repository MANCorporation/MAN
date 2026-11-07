# rclk.py X Y: real right-click at screen pixel (1920x1056)
import json,socket,sys,time,os
s=socket.socket(socket.AF_UNIX); s.connect(os.environ['Q']); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,**({"arguments":a} if a else {})})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
W,H=1920,1056; x,y=int(sys.argv[1]),int(sys.argv[2])
def mv(x,y): cmd("input-send-event",{"events":[{"type":"abs","data":{"axis":"x","value":int(x*32767/W)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/H)}}]})
mv(x-6,y-6); time.sleep(0.15); mv(x,y); time.sleep(0.25)
def btn(d): cmd("input-send-event",{"events":[{"type":"btn","data":{"down":d,"button":"right"}}]})
btn(True); time.sleep(0.08); btn(False)
