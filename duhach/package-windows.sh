#!/usr/bin/env bash
# package-windows.sh EXE VERSION: make the DUHACH! folder and zip for Windows.
# The folder holds DUHACH!.exe, the GTK/libadwaita DLLs it needs (found by
# following imports) and share/ (icons, settings schemas), so it runs on a
# plain Windows 10/11 PC without installing anything.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
exe=$1
version=$2
gtk="$here/../tools/gtk-win/root/ucrt64"
objdump="$here/../tools/mingw/root/usr/bin/x86_64-w64-mingw32-objdump"
out_dir="$here/../release"
stage="$out_dir/.duhach-stage"
name="DUHACH!"
dest="$stage/$name"
rm -rf "$stage"
mkdir -p "$dest"
cp "$exe" "$dest/$name.exe"

# DLLs: everything imported, recursively, that MSYS2 provides.
declare -A seen
queue=("$dest/$name.exe")
loaders_dir="$gtk/lib/gdk-pixbuf-2.0/2.10.0/loaders"
for l in png svg jpeg; do
    f="$loaders_dir/libpixbufloader-$l.dll"
    [[ -f $f ]] || f="$loaders_dir/pixbufloader_$l.dll"
    [[ -f $f ]] && queue+=("$f")
done
queue+=("$gtk/bin/gdbus.exe")
while ((${#queue[@]})); do
    f=${queue[0]}
    queue=("${queue[@]:1}")
    for dll in $("$objdump" -p "$f" | sed -n 's/^\s*DLL Name: //p'); do
        key=${dll,,}
        [[ -n ${seen[$key]:-} ]] && continue
        seen[$key]=1
        src="$gtk/bin/$dll"
        if [[ -f $src ]]; then
            cp "$src" "$dest/"
            queue+=("$src")
        fi
    done
done
cp "$gtk/bin/gdbus.exe" "$dest/"

# Pixbuf loaders (app icon and pictures) with a cache using relative paths.
mkdir -p "$dest/lib/gdk-pixbuf-2.0/2.10.0/loaders"
for f in "$loaders_dir"/*.dll; do
    case $(basename "$f") in *png*|*svg*|*jpeg*) cp "$f" "$dest/lib/gdk-pixbuf-2.0/2.10.0/loaders/" ;; esac
done
cp "$here/data/windows/loaders.cache" "$dest/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"

# Data: settings schemas (compiled), icons.
mkdir -p "$dest/share/glib-2.0/schemas" "$dest/share/icons"
cp "$gtk/share/glib-2.0/schemas/"org.gtk.*.xml "$dest/share/glib-2.0/schemas/" 2>/dev/null || true
cp "$gtk/share/glib-2.0/schemas/gschema.dtd" "$dest/share/glib-2.0/schemas/" 2>/dev/null || true
glib-compile-schemas "$dest/share/glib-2.0/schemas"
cp -r "$gtk/share/icons/Adwaita" "$gtk/share/icons/hicolor" "$dest/share/icons/"
cp -r "$here/data/icons/hicolor/." "$dest/share/icons/hicolor/"
cp "$here/data/windows/README.txt" "$dest/README.txt"

# Licenses: DUHACH! itself, its Rust libraries, and the MSYS2 libraries
# bundled above (LGPL and others: their texts plus where the source is).
cp "$here/../LICENSE" "$dest/LICENSE.txt"
cp "$here/THIRD-PARTY-RUST.txt" "$dest/THIRD-PARTY-RUST.txt"
cp -r "$gtk/share/licenses" "$dest/licenses"
{
    echo "Third-party libraries bundled with DUHACH!"
    echo "=========================================="
    echo
    echo "The DLLs, icons and data next to DUHACH!.exe come unmodified from these"
    echo "MSYS2 packages (https://www.msys2.org). Each keeps its own license; the"
    echo "license files are in the 'licenses' folder. GTK, GLib, libadwaita, Pango,"
    echo "gdk-pixbuf, librsvg and others are under the GNU LGPL: you may replace"
    echo "their DLLs with your own builds. Their complete source code is available"
    echo "at https://github.com/msys2/MINGW-packages (build recipes) and"
    echo "https://repo.msys2.org/mingw/sources/ (source archives, same versions),"
    echo "and MAN Corporation will provide a copy on request for three years after"
    echo "this release (see https://github.com/MANCorporation/MAN/issues)."
    echo
    ls "$here/../tools/gtk-win/pkgs" | sed -n 's/^mingw-w64-ucrt-x86_64-\(.*\)-any\.pkg\.tar\.zst$/  \1/p'
} > "$dest/THIRD-PARTY-NOTICES.txt"
cp "$here/COPYING" "$dest/COPYING.txt"

mkdir -p "$out_dir"
zip_path="$out_dir/$name-$version-windows-x64.zip"
rm -f "$zip_path"
(cd "$stage" && 7z a -tzip -mx=9 -bd -bso0 "$zip_path" "$name")
rm -rf "$stage"
(cd "$out_dir" && sha256sum "$(basename "$zip_path")" > "$(basename "$zip_path").sha256")
echo "Built $zip_path ($(du -h "$zip_path" | cut -f1))"
