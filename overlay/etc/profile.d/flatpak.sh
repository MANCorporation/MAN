#!/bin/sh
# Make applications installed by Flatpak visible to COSMIC and shell tools.
flatpak_user_exports="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/exports/share"
flatpak_system_exports=/var/lib/flatpak/exports/share
case ":${XDG_DATA_DIRS:-}:" in
    *":$flatpak_user_exports:"*) ;;
    *) XDG_DATA_DIRS="$flatpak_user_exports:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" ;;
esac
case ":$XDG_DATA_DIRS:" in
    *":$flatpak_system_exports:"*) ;;
    *) XDG_DATA_DIRS="$flatpak_system_exports:$XDG_DATA_DIRS" ;;
esac
export XDG_DATA_DIRS
