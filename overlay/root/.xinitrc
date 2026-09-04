# MAN Desktop session — launched by `startx` (Xorg fallback) or by S99man-desktop (Wayland/COSMIC).
# MAN Desktop is a rebranded build of the COSMIC desktop environment.

export MAN_DESKTOP=1

# Start COSMIC's wallpaper daemon (sets the desktop background)
/usr/bin/cosmic-bg &

# Start the notification daemon
/usr/bin/cosmic-notifications &

# Start the panel
/usr/bin/cosmic-panel &

# Start the file manager (shows desktop icons)
/usr/bin/cosmic-files --daemon &

# Finally, start the compositor / session manager
exec cosmic-session
