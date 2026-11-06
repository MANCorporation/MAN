# dockmatch.py stock.ppm dock.ppm X0 X1 Y0 Y1: pixels that differ strongly (icons, not tint)
# inside the box, and where the bar's edges are on the middle row/column.
import sys
def load(f):
    d=open(f,'rb').read(); p=d.split(b'\n',3); W,H=map(int,p[1].split()); return W,H,p[3]
W,H,a=load(sys.argv[1]); _,_,b=load(sys.argv[2]); x0,x1,y0,y1=map(int,sys.argv[3:7])
pix=lambda d,x,y:d[(y*W+x)*3:(y*W+x)*3+3]
xs=[];ys=[]
for y in range(y0,y1):
    for x in range(x0,x1):
        if sum(abs(c-e) for c,e in zip(pix(a,x,y),pix(b,x,y)))>70: xs.append(x);ys.append(y)
print('strongly different px',len(xs),'box',(min(xs),min(ys),max(xs),max(ys)) if xs else None)
