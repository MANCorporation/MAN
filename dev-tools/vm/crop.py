# crop.py in.ppm out.ppm x y w h scale
import sys
d=open(sys.argv[1],'rb').read(); p=d.split(b'\n',3); W,H=map(int,p[1].split()); px=p[3]
x,y,w,h,s=map(int,sys.argv[3:8]); out=bytearray()
for j in range(h):
    row=bytearray()
    for i in range(w): row+=px[((y+j)*W+x+i)*3:((y+j)*W+x+i)*3+3]*s
    out+=bytes(row)*s
open(sys.argv[2],'wb').write(b'P6\n%d %d\n255\n'%(w*s,h*s)+bytes(out))
