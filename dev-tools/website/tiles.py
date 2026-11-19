#!/usr/bin/env python3
# tiles.py FULL.png OUT.png [N]: a tall page screenshot cut into N columns side by side.
import base64, os, struct, subprocess, sys
src, out = sys.argv[1], sys.argv[2]
n = int(sys.argv[3]) if len(sys.argv) > 3 else 4
data = open(src, 'rb').read()
w, h = struct.unpack('>II', data[16:24])
th = (h + n - 1) // n
d = base64.b64encode(data).decode()
parts = [f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w * n}" height="{th}">',
         f'<defs><image id="p" width="{w}" height="{h}" xlink:href="data:image/png;base64,{d}"/></defs>']
for i in range(n):
    parts.append(f'<svg x="{i * w}" y="0" width="{w}" height="{th}" viewBox="0 {i * th} {w} {th}"><use xlink:href="#p"/></svg>')
parts.append('</svg>')
open(out + '.svg', 'w').write(''.join(parts))
subprocess.run([os.path.expanduser('~/Documents/MANRUST/tools/rsvg/rsvg-convert'), '-w', '2000', out + '.svg', '-o', out], check=True)
os.remove(out + '.svg')
