# MAN Distribution Manifest

This file describes the MAN Linux Distribution package manifest.

## Packages

### Core System
- linux (kernel 6.6.x)
- glibc (C library)
- busybox (rescue shell & utilities)

### Rust Ecosystem
- rustc (Rust compiler on target)
- cargo (package manager on target)
- rust-std-aarch64-unknown-linux-gnu
- rust-std-x86_64-unknown-linux-gnu
- uutils/coreutils (Rust reimplementation of GNU coreutils)

### Shells
- bash
- zsh

### Networking
- openssh
- iproute2
- iptables
- net-tools
- dhcp
- dhcpcd

### Filesystem & Disk
- util-linux
- e2fsprogs

### System Tools
- strace
- procps-ng
- pciutils
- vim
- nano
- tmux

## Default Services
- sshd (SSH daemon)
- ntpd (NTP client)

## Default Users
- root (login: root, no password by default)
