# imgdiff.py a.ppm b.ppm: how many pixels of the bottom 110 rows differ, and where
import sys
def load(f):
    d=open(f,'rb').read(); p=d.split(b'\n',3); W,H=map(int,p[1].split()); return W,H,p[3]
W,H,a=load(sys.argv[1]); _,_,b=load(sys.argv[2]); xs=[];ys=[]
for y in range(H-110,H):
    for x in range(W):
        i=(y*W+x)*3
        if abs(a[i]-b[i])+abs(a[i+1]-b[i+1])+abs(a[i+2]-b[i+2])>24: xs.append(x);ys.append(y)
print('diff px',len(xs),'box',(min(xs),min(ys),max(xs),max(ys)) if xs else None)
