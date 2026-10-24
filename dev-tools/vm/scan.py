# scan.py image out.tsv : walk ext4 image via debugfs `ls -p`, emit path\tmode(octal)\tuid\tgid
import subprocess,sys
img,out=sys.argv[1],sys.argv[2]
rows=[]; queue=['/']
while queue:
    batch,queue=queue[:2000],queue[2000:]
    cmds="".join(f'ls -p "{d}"\n' for d in batch)
    r=subprocess.run(['/sbin/debugfs','-f','-',img],input=cmds,capture_output=True,text=True,errors='replace')
    cur=None; it=iter(batch)
    for line in r.stdout.splitlines():
        if line.startswith('debugfs:'):
            cur=next(it); continue
        p=line.split('/')
        if len(p)<7 or cur is None: continue
        ino,mode,uid,gid,name=p[1],p[2],p[3],p[4],'/'.join(p[5:-2])
        if name in ('.','..'): continue
        path=(cur.rstrip('/')+'/'+name)
        m=int(mode,8)
        rows.append(f"{path}\t{mode}\t{uid}\t{gid}")
        if (m & 0o170000)==0o040000 and path not in ('/proc','/sys','/dev') and name!='lost+found':
            queue.append(path)
open(out,'w').write("\n".join(rows)+"\n")
print(len(rows))
