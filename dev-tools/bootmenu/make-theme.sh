#!/bin/bash
# Builds MAN's boot menu theme into man-installer/data/grub/theme/man:
# icons (make-icons.py), the selection box pieces and GRUB fonts made from
# Adwaita Sans (SIL OFL 1.1, fonts/AdwaitaSans-Regular.ttf from the MAN image).
# theme.txt and advanced.txt are kept by hand in that folder.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
out=$here/../../man-installer/data/grub/theme/man
rsvg=$HOME/Documents/MANRUST/tools/rsvg/rsvg-convert
mkdir -p "$out/icons"
tmp=$(mktemp -d)
trap 'rm -r "$tmp"' EXIT

python3 "$here/make-icons.py" "$tmp/icons"
cp "$tmp"/icons/*.png "$out/icons/"

# Selection box: a rounded square in nine pieces (GRUB stretches the middle).
# select_*: the picker (radius 22), list_*: the Advanced Options list (12).
box() { # prefix radius fill
    local p=$1 r=$2 fill=$3 s=$(( $2 * 2 + 2 ))
    cat > "$tmp/$p.svg" <<SVG
<svg xmlns="http://www.w3.org/2000/svg" width="$s" height="$s"><rect width="$s" height="$s" rx="$r" fill="$fill"/></svg>
SVG
    "$rsvg" "$tmp/$p.svg" -o "$tmp/$p.png"
    python3 - "$tmp/$p.png" "$out" "$p" "$r" <<'PY'
import sys, zlib, struct
src, out, prefix, r = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
data = open(src, 'rb').read()
# Minimal PNG reader for rsvg's 8-bit RGBA, non-interlaced output.
pos, idat, w, h = 8, b'', 0, 0
while pos < len(data):
    n, t = struct.unpack('>I4s', data[pos:pos+8]); c = data[pos+8:pos+8+n]
    if t == b'IHDR': w, h = struct.unpack('>II', c[:8])
    if t == b'IDAT': idat += c
    pos += 12 + n
raw = zlib.decompress(idat); bpp = 4; stride = w * bpp
rows, prev = [], bytearray(stride)
for y in range(h):
    f = raw[y*(stride+1)]; line = bytearray(raw[y*(stride+1)+1:(y+1)*(stride+1)])
    for i in range(stride):
        a = line[i-bpp] if i >= bpp else 0; b = prev[i]; c = prev[i-bpp] if i >= bpp else 0
        if f == 1: line[i] = (line[i] + a) & 255
        elif f == 2: line[i] = (line[i] + b) & 255
        elif f == 3: line[i] = (line[i] + (a + b) // 2) & 255
        elif f == 4:
            p = a + b - c; pa, pb, pc = abs(p-a), abs(p-b), abs(p-c)
            line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
    rows.append(bytes(line)); prev = line
def save(name, x0, y0, cw, ch):
    body = b''.join(b'\0' + rows[y][x0*4:(x0+cw)*4] for y in range(y0, y0+ch))
    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d))
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', cw, ch, 8, 6, 0, 0, 0)) \
        + chunk(b'IDAT', zlib.compress(body, 9)) + chunk(b'IEND', b'')
    open(f'{out}/{prefix}_{name}.png', 'wb').write(png)
m = w - 2 * r
for name, x, y, cw, ch in [('nw', 0, 0, r, r), ('n', r, 0, m, r), ('ne', r+m, 0, r, r),
                           ('w', 0, r, r, m), ('c', r, r, m, m), ('e', r+m, r, r, m),
                           ('sw', 0, r+m, r, r), ('s', r, r+m, m, r), ('se', r+m, r+m, r, r)]:
    save(name, x, y, cw, ch)
PY
}
box select 22 'rgba(255,255,255,0.20)'
box list 12 'rgba(255,255,255,0.14)'

for s in 14 18 24 32; do
    grub-mkfont -s $s -o "$out/adwaita-sans-$s.pf2" "$here/fonts/AdwaitaSans-Regular.ttf" 2>/dev/null
done
echo "Theme assets in $out"
