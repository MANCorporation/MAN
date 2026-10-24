import socket,sys,json,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw'); f.readline()
def cmd(c,a=None):
    f.write(json.dumps({"execute":c,"arguments":a} if a else {"execute":c})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd("qmp_capabilities"); x,y=float(sys.argv[2]),float(sys.argv[3])
ev=lambda *e: cmd("input-send-event",{"events":list(e)})
ev({"type":"abs","data":{"axis":"x","value":int(x*32767/1280)}},{"type":"abs","data":{"axis":"y","value":int(y*32767/800)}}); time.sleep(0.15)
for _ in range(2):
    ev({"type":"btn","data":{"down":True,"button":"left"}}); time.sleep(0.04); ev({"type":"btn","data":{"down":False,"button":"left"}}); time.sleep(0.08)
