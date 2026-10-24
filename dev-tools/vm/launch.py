# launch.py MODE NAME : start run-gnome.sh MODE's exact qemu args with scratch copies; headless + monitor/qmp
import subprocess,sys,os
S=os.path.dirname(os.path.abspath(__file__)); P='/home/kristihack/Documents/MANRUST'; L=P+'/lfs'
mode,name=sys.argv[1],sys.argv[2]
raw=subprocess.run(['bash','-c','exec() { printf "%s\\0" "$@"; }; source ./run-gnome.sh '+mode],cwd=P,capture_output=True).stdout
args=raw.decode().split('\0')[:-1]
rep={'file='+L+'/install-target.qcow2,format=qcow2':'file='+os.environ.get('TGTDIR',S)+'/'+os.environ.get('TGT','t-target.qcow2')+',format=qcow2','file='+L+'/man-gnome.img,format=raw':'file='+S+'/'+os.environ.get('LIVE','t-live.qcow2')+',format=qcow2', L+'/ovmf-vars.fd':S+'/t-vars.fd',
     L+'/install-target.qcow2':S+'/t-target.qcow2'}
out=[]
for a in args:
    for k,v in rep.items(): a=a.replace(k,v)
    out.append(a)
i=out.index('-display'); out[i+1]='none'
i=out.index('-serial'); out[i+1]='null'
os.chdir(S); out+=['-monitor',f'unix:{name}.mon,server,nowait','-qmp',f'unix:{name}.qmp,server,nowait']
bad=[a for a in out if L in a and not (a.endswith('vmlinuz-7.3-rc7') or 'man-live-boot.img' in a or L+'/.vmtest/' in a or 'updates-stick.img' in a)]
assert not bad, bad
assert mode=='installed' or any('readonly=on' in a and 'man-live-boot' in a for a in out)
subprocess.Popen(out, stdout=open(f'{name}.err','w'), stderr=subprocess.STDOUT, start_new_session=True)
print('started', name)
