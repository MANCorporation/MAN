#!/usr/bin/env python3
# montage.py OUT.png BG IN.png...: icons (256 px) side by side, for a quick look.
import sys, subprocess, base64, os
out, bg, files = sys.argv[1], sys.argv[2], sys.argv[3:]
S = 256; pad = 16
w = len(files) * (S + pad) + pad; h = S + 2 * pad
p = [f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}"><rect width="100%" height="100%" fill="{bg}"/>']
for i, f in enumerate(files):
    d = base64.b64encode(open(f, 'rb').read()).decode()
    p.append(f'<image x="{pad + i * (S + pad)}" y="{pad}" width="{S}" height="{S}" xlink:href="data:image/png;base64,{d}"/>')
open(out + '.svg', 'w').write(''.join(p) + '</svg>')
subprocess.run([os.path.expanduser('~/Documents/MANRUST/tools/rsvg/rsvg-convert'), out + '.svg', '-o', out], check=True)
os.remove(out + '.svg')
