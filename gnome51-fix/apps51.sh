#!/bin/bash
# Make every preinstalled app a GNOME 51 app; remove GNOME 48 leftovers.
# Runs INSIDE the guest as root. $1 = directory with the local repo (Packages + .debs).
set -u
R=${1:-/mnt/repo}; D=$(dirname "$0")
echo "deb [trusted=yes] file:$R ./" > /root/man-local.list
A=(-o Dir::Etc::SourceList=/root/man-local.list -o Dir::Etc::SourceParts=/nonexistent
   -o Dir::Etc::Preferences=/nonexistent -o Dir::Etc::PreferencesParts=/nonexistent)
export DEBIAN_FRONTEND=noninteractive
# hostname(1) was never installed; ssl-cert/cups postinst need it
dpkg -s hostname >/dev/null 2>&1 || dpkg -i "$D"/hostname_*.deb || exit 1
apt-get "${A[@]}" update -q || exit 1
RM=(); for x in $(cat "$D/apps51-remove.txt"); do dpkg -s "${x%-}" >/dev/null 2>&1 && RM+=("$x"); done
echo "== 1/3 register GNOME 51 overlay, install GNOME 51 apps, remove GNOME 48 leftovers"
apt-get "${A[@]}" install -y -q --allow-downgrades -o Dpkg::Options::=--force-confold \
  $(cat "$D/apps51-overlay-pins.txt") $(cat "$D/apps51-install.txt") "${RM[@]}" || exit 1
echo "== 2/3 delete unregistered GNOME 48 app files (evince, totem, gnome-terminal)"
for deb in "$R"/evince_* "$R"/evince-common_* "$R"/libevdocument3-* "$R"/libevview3-* \
           "$R"/totem_* "$R"/totem-common_* "$R"/gnome-terminal_* "$R"/gnome-terminal-data_*; do
  dpkg-deb --fsys-tarfile "$deb" | tar -t | sed 's|^\.||' | grep -v '/$'
done | sort -u | while read -r f; do
  [[ -e $f || -L $f ]] || continue
  dpkg -S "$f" >/dev/null 2>&1 && continue          # owned by an installed package: keep
  rm -f "$f" && echo "removed $f"
done | tee /root/apps51-removed-orphans.txt | wc -l
find /usr/share/help /usr/lib/x86_64-linux-gnu/totem /usr/share/totem /usr/share/evince /usr/libexec/gnome-terminal* -depth -type d -empty -delete 2>/dev/null
echo "== 3/3 refresh caches"
update-desktop-database -q /usr/share/applications; gtk-update-icon-cache -qf /usr/share/icons/hicolor 2>/dev/null
glib-compile-schemas /usr/share/glib-2.0/schemas; ldconfig
apt-get "${A[@]}" -s -o Debug::NoLocking=1 check && echo "apt: dependency state consistent"
rm -f /root/man-local.list
echo "apps51: done"
