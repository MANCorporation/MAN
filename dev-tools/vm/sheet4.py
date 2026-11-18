# sheet4.py OUT.png A.png B.png C.png D.png: four 800x600 screenshots in a 2x2 sheet.
import base64, subprocess, os, sys
out, fs = sys.argv[1], sys.argv[2:]
p = ['<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1620" height="1220"><rect width="100%" height="100%" fill="#444"/>']
for i, f in enumerate(fs):
    d = base64.b64encode(open(f, 'rb').read()).decode()
    p.append(f'<image x="{10+(i%2)*810}" y="{10+(i//2)*610}" width="800" height="600" xlink:href="data:image/png;base64,{d}"/>')
open(out + '.svg', 'w').write(''.join(p) + '</svg>')
subprocess.run([os.path.expanduser('~/Documents/MANRUST/tools/rsvg/rsvg-convert'), out + '.svg', '-o', out], check=True)
os.remove(out + '.svg')
