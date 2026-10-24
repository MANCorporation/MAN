import os,sys,subprocess,glob,collections,urllib.parse
dirs=["/home/kristihack/Documents/MAN/work/apt-gnome51-amd64/archives","/home/kristihack/Documents/MAN/artifacts/debs/amd64","/home/kristihack/Documents/MAN/work/rootfs-amd64-gnome51/var/cache/apt/archives"]
idx={}
for d in dirs:
    for f in glob.glob(d+"/*.deb"):
        b=os.path.basename(f)[:-4].split('_')
        if len(b)<3: continue
        idx.setdefault((b[0],urllib.parse.unquote(b[1])),f)
bypkg=collections.defaultdict(list)
for (p,v),f in idx.items(): bypkg[p].append((v,f))
overlay=dict(l.split('\t')[:2] for l in open(sys.argv[2]).read().split('\n') if '\t' in l)
want=[]; missing=[]
for l in open(sys.argv[1]):
    p,v,a=l.rstrip('\n').split('\t'); p0=p
    v=overlay.get(p,v)
    f=idx.get((p,v)) or idx.get((p,v.split(':',1)[-1]))
    if not f and bypkg.get(p): f=sorted(bypkg[p])[-1][1]
    (want.append(f) if f else missing.append(p))
res={}
for f in want:
    t=subprocess.run(f"dpkg-deb --fsys-tarfile '{f}' | tar -tv --quoting-style=literal",shell=True,capture_output=True,text=True).stdout
    for line in t.splitlines():
        parts=line.split(None,5)
        if len(parts)<6: continue
        perm,own=parts[0],parts[1]
        name=parts[5].split(' -> ')[0].split(' link to ')[0]
        if name.startswith('./'): name=name[1:]
        name=name.rstrip('/') or '/'
        special = own!='root/root' or any(c in perm for c in 'sStT')
        if special and perm[0] in '-d': res[name]=(perm,own,os.path.basename(f))
for k in sorted(res): print(k,*res[k],sep='\t')
print(f"# debs={len(want)} missing={len(missing)}: {' '.join(missing)}",file=sys.stderr)
