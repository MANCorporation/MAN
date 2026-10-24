# burst.py qmp X Y N interval prefix : click at X,Y then take N screendumps every interval seconds
import socket,sys,json,time
sock,x,y,n,iv,pre=sys.argv[1],float(sys.argv[2]),float(sys.argv[3]),int(sys.argv[4]),float(sys.argv[5]),sys.argv[6]
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,"arguments":a} if a else {"execute":c})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
ev=lambda *e: cmd("input-send-event",{"events":list(e)})
ev({"type":"abs","data":{"axis":"x","value":int(x*32767/1280)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/800)}}); time.sleep(0.1)
ev({"type":"btn","data":{"down":True,"button":"left"}}); time.sleep(0.05); ev({"type":"btn","data":{"down":False,"button":"left"}})
t0=time.time()
for i in range(n):
    cmd("screendump",{"filename":f"{pre}{i}.ppm"}); print(f"{i}: {time.time()-t0:.2f}s"); time.sleep(iv)
