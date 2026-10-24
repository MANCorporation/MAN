#!/bin/bash
# gcc.sh PANEL : open Settings on PANEL in the tw VM as MANInstaller
S=$(dirname "$0"); cd $S
python3 -I ser.py tw.dbg "pkill -f gnome-control-center; sleep 1; (runuser -u MANInstaller -- env XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus WAYLAND_DISPLAY=wayland-0 XDG_CURRENT_DESKTOP=GNOME gnome-control-center $1 > /tmp/gcc.log 2>&1 &)" 3 >/dev/null
