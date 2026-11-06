# dockbox.py a.ppm b.ppm: compares the bottom 130 rows of two screenshots of the same wallpaper:
# prints the box of different pixels, and with one file the column/row profile is skipped.
import sys
def load(f):
    d=open(f,'rb').read(); p=d.split(b'\n',3); W,H=map(int,p[1].split()); return W,H,p[3]
W,H,a=load(sys.argv[1]); _,_,b=load(sys.argv[2]); xs=[];ys=[];t=0
for y in range(H-130,H):
    for x in range(W):
        i=(y*W+x)*3; d=abs(a[i]-b[i])+abs(a[i+1]-b[i+1])+abs(a[i+2]-b[i+2])
        if d>12: xs.append(x);ys.append(y)
print('different px',len(xs),'box',(min(xs),min(ys),max(xs),max(ys)) if xs else None)
