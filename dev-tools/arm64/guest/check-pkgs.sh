#!/bin/bash
# Which of the x86 MAN packages exist in Debian sid for arm64 (separate apt state).
set -e
D=/root/sid; mkdir -p $D/lists/partial $D/cache/partial $D/etc; : > $D/status
echo "deb [arch=arm64] http://deb.debian.org/debian sid main contrib non-free non-free-firmware" > $D/etc/sources.list
A=(-o Dir::Etc::SourceList=$D/etc/sources.list -o Dir::Etc::SourceParts=/nonexistent -o Dir::State::Lists=$D/lists -o Dir::Cache=$D/cache -o APT::Architecture=arm64 -o Acquire::ForceIPv4=true -o Dir::State::status=$D/status)
[ -n "${NOUPDATE:-}" ] || apt-get "${A[@]}" update -q >/dev/null
apt-cache "${A[@]}" pkgnames > $D/all.txt
sort -u $D/all.txt > $D/all.sorted
cat /mnt/share/pkgs-x86.txt /mnt/share/pkgs-extra.txt | sort -u > $D/want.txt
# Real packages with an installable version (pkgnames also lists virtual ones);
# never x86 cross tools that happen to exist for arm64 too.
grep -v -E 'x86-64|amd64' $D/want.txt > $D/want2.txt
apt-cache "${A[@]}" policy $(cat $D/want2.txt) 2>/dev/null | awk '/^[^ ].*:$/{p=substr($1,1,length($1)-1)} /Candidate:/{ if ($2!="(none)") print p }' | sort -u > /mnt/share/pkgs-arm64.txt
comm -23 $D/want.txt /mnt/share/pkgs-arm64.txt > /mnt/share/pkgs-missing.txt
wc -l /mnt/share/pkgs-arm64.txt /mnt/share/pkgs-missing.txt
