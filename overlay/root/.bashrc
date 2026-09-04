# Root .bashrc for MAN Linux
# Sourced by bash for interactive non-login shells.

# History settings
HISTCONTROL=ignoreboth
HISTSIZE=1000
HISTFILESIZE=2000

# Don't put duplicate lines or lines starting with space in the history
HISTCONTROL=ignoreboth:erasedups

# Append to the history file, don't overwrite
shopt -s histappend

# Check window size after each command and update LINES/COLUMNS
shopt -s checkwinsize

# Set a colorful prompt
PS1='\[\033[01;36m\]\u@man\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '

# MAN aliases
alias ll='ls -la --color=auto'
alias la='ls -A --color=auto'
alias l='ls -CF --color=auto'
alias ..='cd ..'
alias ...='cd ../..'
