# MAN Linux Distribution — environment setup
# This file is sourced by /etc/profile for all login shells.

# --- System locale ----------------------------------------------------------
# /etc/default/locale is written by man-utilities when the user confirms
# their language choice in the language chooser.  Sourcing it here ensures
# that every login shell (and all child processes) inherit the correct
# LANG/LC_* variables so the *entire OS* is translated, not just
# man-utilities' own UI.
if [ -r /etc/default/locale ]; then
    . /etc/default/locale 2>/dev/null || true
fi

# Add Rust toolchain (if present in the image) to PATH
if [ -d /usr/local/rustup ]; then
    export RUSTUP_HOME=/usr/local/rustup
    export CARGO_HOME=/usr/local/cargo
    export PATH="$CARGO_HOME/bin:$PATH"
fi

# MAN-specific aliases
alias ll='ls -la --color=auto'
alias la='ls -A --color=auto'
alias l='ls -CF --color=auto'
alias man-update='cargo install --path /usr/src/man-cli 2>/dev/null || echo "MAN CLI not found in this image"'

# MOTD is shown by login (busybox motd()) or sshd (PrintMotd=yes).
# Not shown here to avoid duplicate display on auto-login.

# Set prompt
if [ -n "$BASH" ]; then
    PS1='\[\033[01;36m\]\u@man\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '
elif [ -n "$ZSH_VERSION" ]; then
    PROMPT='%F{cyan}man%f:%F{blue}%~%f %# '
fi
