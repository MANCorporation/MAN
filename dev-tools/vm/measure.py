import sys
d=open(sys.argv[1],'rb').read(); p=d.split(b'\n',3); W,H=map(int,p[1].split()); px=p[3]
x=int(sys.argv[2]); prev=None
for y in range(int(sys.argv[3]),H):
    r=px[(y*W+x)*3:(y*W+x)*3+3]
    if prev and sum(abs(a-b) for a,b in zip(r,prev))>int(sys.argv[4]): print(y,tuple(prev),tuple(r))
    prev=r
