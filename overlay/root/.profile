# Root .profile for MAN Linux
# Sourced by login shells.

# Source .bashrc for interactive shells
if [ -n "$BASH" ] && [ -f ~/.bashrc ]; then
    . ~/.bashrc
fi

# Rust environment
if [ -d /usr/local/cargo ]; then
    export CARGO_HOME=/usr/local/cargo
    export RUSTUP_HOME=/usr/local/rustup
    export PATH="$CARGO_HOME/bin:$PATH"
fi

# MAN distro banner (short)
echo "┌── MAN Linux v0.1.0 — Welcome ──┐"
echo "│  Arch: $(uname -m)              │"
echo "│  Kernel: $(uname -r)            │"
echo "│  Type 'help' for available cmds │"
echo "└────────────────────────────────┘"
