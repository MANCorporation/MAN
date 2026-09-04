# MAN Linux Distribution — COSMIC minimal theme configuration.
#
# Simple visual style: dark theme, plain wallpaper, and minimal panel chrome.

# Use the dark color scheme (COSMIC's default, enforced here for clarity).
export COSMIC_PREFERRED_COLOR_SCHEME=dark

# Use a plain solid-color wallpaper instead of a photo.
export COSMIC_BACKGROUND_URI=file:///usr/share/backgrounds/cosmic/man-simple.png

# Minimal accent color: soft blue that matches MAN branding.
export COSMIC_ACCENT_COLOR=#4a9eff

# Motion is enabled by default. Users and accessibility tooling can set
# MAN_REDUCED_MOTION=1 before starting COSMIC to disable transitions globally.
export MAN_REDUCED_MOTION="${MAN_REDUCED_MOTION:-0}"
if [ "$MAN_REDUCED_MOTION" = 1 ]; then
    export COSMIC_DISABLE_ANIMATIONS=true
else
    export COSMIC_DISABLE_ANIMATIONS=false
fi
# A stable, documented policy value for MAN components that implement their
# own transitions. Values: 0.5 (fast), 1 (normal), 1.5 (slow).
export MAN_ANIMATION_SPEED="${MAN_ANIMATION_SPEED:-1}"

# Single simple workspace — no workspace bar or switching effects.
export COSMIC_WORKSPACE_COUNT=1

# Panel: bottom edge only, auto-hide after idle, minimal applets.
export COSMIC_PANEL_POSITION=Bottom
export COSMIC_PANEL_AUTO_HIDE=1
export COSMIC_PANEL_DESKTOP_APPS=false
