#!/bin/bash
# Images for MAN's start-up screen (Plymouth theme man-installer/data/plymouth/man):
# the logo (from man-installer/data/logo/man-logo-v2.svg), the progress bar's
# pieces, the password field and its dots.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
p=$(cd "$here/../.." && pwd)
out=$p/man-installer/data/plymouth/man
rsvg=$p/tools/rsvg/rsvg-convert
tmp=$(mktemp -d)
trap 'rm -r "$tmp"' EXIT
png() { printf '%s' "$2" > "$tmp/$1.svg"; "$rsvg" "$tmp/$1.svg" -o "$out/$1.png"; }

# The logo, cropped to the shape (the source is a 4096 square).
sed 's#viewBox="0 0 4096 4096"#viewBox="0 642 4096 2811"#; s#width="512" height="512"#width="746" height="512"#' \
    "$p/man-installer/data/logo/man-logo-v2.svg" > "$tmp/logo.svg"
"$rsvg" "$tmp/logo.svg" -o "$out/logo.png"

track='#3a3a3c' fill='#d0d0d0'
# Round ends (half circles, 32x64) and a 1x64 middle that Plymouth stretches.
png cap-left "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='64'><circle cx='32' cy='32' r='32' fill='$fill'/></svg>"
png cap-right "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='64'><circle cx='0' cy='32' r='32' fill='$fill'/></svg>"
png track-left "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='64'><circle cx='32' cy='32' r='32' fill='$track'/></svg>"
png track-right "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='64'><circle cx='0' cy='32' r='32' fill='$track'/></svg>"
png fill "<svg xmlns='http://www.w3.org/2000/svg' width='4' height='64'><rect width='4' height='64' fill='$fill'/></svg>"
png track "<svg xmlns='http://www.w3.org/2000/svg' width='4' height='64'><rect width='4' height='64' fill='$track'/></svg>"
# Password field (rounded, like GNOME's entries on a dark background) and a dot.
png field "<svg xmlns='http://www.w3.org/2000/svg' width='560' height='78'><rect x='1' y='1' width='558' height='76' rx='16' fill='#1e1e20' stroke='#5e5c64' stroke-width='2'/></svg>"
png bullet "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'><circle cx='16' cy='16' r='16' fill='#ffffff'/></svg>"
echo "Splash images in $out"
