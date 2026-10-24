#!/bin/bash
# MAN Linux GNOME 51 repair - runs INSIDE the guest as real root (never on the host).
# Fixes the damage left by rootless (non-root) package extraction.
set -u
D=${1:-/root/gnome51-fix}
log(){ echo "[man-repair] $*"; }

log "1/6 ownership: UID/GID 1000 (build user = 'man') -> root, except /home/man"
find / -xdev \( -path /home/man -o -path /proc -o -path /sys -o -path /dev -o -path /run \) -prune -o \
     -uid 1000 -exec chown -h root {} +
find / -xdev \( -path /home/man -o -path /proc -o -path /sys -o -path /dev -o -path /run \) -prune -o \
     -gid 1000 -exec chgrp -h root {} +
chown -R man:man /home/man

log "2/6 special owners/modes from .deb metadata and maintainer scripts"
sym2oct(){ # -rwsr-xr-x -> 4755
  local p=${1:1} o=0 s=0 i c v
  for i in 0 3 6; do v=0
    [[ ${p:i:1} == r ]] && v=$((v+4)); [[ ${p:i+1:1} == w ]] && v=$((v+2))
    c=${p:i+2:1}; [[ $c == [xst] ]] && v=$((v+1))
    [[ $c == [sS] && $i == 0 ]] && s=$((s+4)); [[ $c == [sS] && $i == 3 ]] && s=$((s+2)); [[ $c == [tT] ]] && s=$((s+1))
    o=$((o*10+v)); done; printf '%d%03d' $s $o; }
while IFS=$'\t' read -r path perm u g; do
  [[ -e $path ]] || continue
  chown "$u:$g" "$path" && chmod "$(sym2oct "$perm")" "$path" && log "  $(stat -c '%A %U:%G' "$path") $path"
done < "$D/special-perms.tsv"
# dpkg statoverrides recorded in the image
[[ -f /var/lib/dpkg/statoverride ]] && while read -r u g m f; do
  [[ -e $f ]] && chown "$u:$g" "$f" && chmod "$m" "$f"; done < /var/lib/dpkg/statoverride

log "3/6 file capabilities (libgstreamer1.0-0 postinst)"
ptp=/usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-ptp-helper
[[ -e $ptp ]] && { setcap cap_sys_nice,cap_net_bind_service,cap_net_admin+ep "$ptp" || chmod u+s "$ptp"; }

log "4/6 install missing login/nologin (Debian trixie 'login' pkg, from util-linux 2.41.5)"
dpkg -i --force-confold "$D"/login_*.deb

log "5/6 recreate tmpfiles/sysusers state with correct owners"
systemd-sysusers >/dev/null 2>&1
systemd-tmpfiles --create --remove >/dev/null 2>&1

log "6/6 admin rights for desktop user"
usermod -aG sudo,netdev,plugdev,audio,video man

log "remaining files owned by uid 1000 outside /home/man: $(find / -xdev \( -path /home/man -o -path /proc -o -path /sys -o -path /dev -o -path /run \) -prune -o \( -uid 1000 -o -gid 1000 \) -print | grep -vc '^/var/cache/man$')"
log "done"
