#!/bin/bash
# Create the MAN live user and make GDM log it in automatically.
# Runs INSIDE the guest as root.
set -eu
U=MANInstaller
if ! id "$U" >/dev/null 2>&1; then
  useradd -m -c "MAN Installer" -s /bin/bash "$U" 2>/dev/null \
    || useradd --badname -m -c "MAN Installer" -s /bin/bash "$U"
fi
usermod -aG sudo,audio,video,plugdev,netdev "$U"
passwd -d "$U"                                    # live user: no password
printf '%s ALL=(ALL) NOPASSWD:ALL\n' "$U" > /etc/sudoers.d/90-maninstaller
chmod 0440 /etc/sudoers.d/90-maninstaller
visudo -cq
H=$(getent passwd "$U" | cut -d: -f6)
install -d -o "$U" -g "$U" -m 0700 "$H/.config"
echo yes > "$H/.config/gnome-initial-setup-done"; chown "$U:$U" "$H/.config/gnome-initial-setup-done"
# GDM autologin
C=/etc/gdm3/daemon.conf
sed -i '/^AutomaticLoginEnable=/d;/^AutomaticLogin=/d' "$C"
sed -i "s/^\[daemon\]$/[daemon]\nAutomaticLoginEnable=true\nAutomaticLogin=$U/" "$C"
grep -A2 '^\[daemon\]' "$C"; id "$U"
echo "live-user: done"
