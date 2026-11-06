import json,socket,sys,time,os
s=socket.socket(socket.AF_UNIX); s.connect(os.environ['Q']); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,**({"arguments":a} if a else {})})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities")
x,y,n=map(int,sys.argv[1:4]); w="wheel-down" if n>0 else "wheel-up"; n=abs(n)
cmd("input-send-event",{"events":[{"type":"abs","data":{"axis":"x","value":int(x*32767/1920)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/1056)}}]}); time.sleep(.3)
for i in range(n):
    cmd("input-send-event",{"events":[{"type":"btn","data":{"down":True,"button":w}}]}); cmd("input-send-event",{"events":[{"type":"btn","data":{"down":False,"button":w}}]}); time.sleep(.05)
