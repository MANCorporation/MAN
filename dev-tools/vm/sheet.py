# sheet.py out.png in1.ppm ... : tile PPMs (downscaled 2x) into a 3-column PNG
import sys,zlib,struct
def rd(p):
    d=open(p,'rb').read(); parts=d.split(b'\n',3); w,h=map(int,parts[1].split()); return w,h,parts[3]
ims=[rd(p) for p in sys.argv[2:]]; w,h=ims[0][0]//2,ims[0][1]//2; cols=3; rows=(len(ims)+cols-1)//cols
W,H=w*cols,h*rows; out=bytearray(W*H*3)
for i,(iw,ih,px) in enumerate(ims):
    ox,oy=(i%cols)*w,(i//cols)*h
    for y in range(h):
        row=px[(2*y)*iw*3:(2*y+1)*iw*3]
        line=b''.join(row[x*6:x*6+3] for x in range(w))
        out[((oy+y)*W+ox)*3:((oy+y)*W+ox+w)*3]=line
raw=b''.join(b'\0'+bytes(out[y*W*3:(y+1)*W*3]) for y in range(H))
def ch(t,b): return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b))
open(sys.argv[1],'wb').write(b'\x89PNG\r\n\x1a\n'+ch(b'IHDR',struct.pack('>IIBBBBB',W,H,8,2,0,0,0))+ch(b'IDAT',zlib.compress(raw))+ch(b'IEND',b''))
