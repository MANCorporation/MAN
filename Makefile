# MAN Linux Distribution — top-level Makefile
# Convenience wrapper around the `man` Rust CLI.
#
# Usage:
#   make setup    — install deps & clone Toolchain
#   make build    — build both architectures
#   make test     — boot x86_64 in QEMU
#   make test-aarch64 — boot aarch64 in QEMU
#   make disk      — create UEFI disk images for both arches
#   make clean    — clean all build artifacts
#   make iso      — create ISO images from existing builds
#   make release  — build and package ISO images for both architectures

.PHONY: all setup build build-x86_64 build-aarch64 release cosmic cosmic-aarch64 cosmic-x86_64 test test-x86_64 test-aarch64 iso iso-x86_64 iso-aarch64 disk disk-x86_64 disk-aarch64 clean cli info help check

# Project root is the directory containing this Makefile
ROOT_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))

# The Rust CLI binary
CLI_BIN := $(ROOT_DIR)/man/target/release/man

# Default target
all: cli

# Build the Rust CLI (release mode)
cli:
	@echo "🔨 Building MAN CLI (release)..."
	cd $(ROOT_DIR)/man && cargo build --release

# Run the CLI directly
man: cli
	$(CLI_BIN) $(RUN_ARGS)

# Setup: install deps and clone Toolchain
setup:
	$(CLI_BIN) setup --install-deps --all-archs

# Build both architectures
build:
	$(CLI_BIN) build --arch x86-64
	$(CLI_BIN) build --arch aarch64

# Build individual architectures
build-x86_64:
	$(CLI_BIN) build --arch x86-64

build-aarch64:
	$(CLI_BIN) build --arch aarch64

# Build both targets and package their UEFI ISO images. The CLI builds the
# targets in sequence so each can use the selected CPU capacity and shared
# caches without exhausting host memory.
release: cli
	$(CLI_BIN) release

# Rebuild the desktop for both architectures. `man build` already includes
# COSMIC; use these targets when iterating on desktop-only changes.
cosmic: cosmic-x86_64 cosmic-aarch64

cosmic-aarch64: cli
	$(CLI_BIN) cosmic --arch aarch64

cosmic-x86_64: cli
	$(CLI_BIN) cosmic --arch x86-64

# Test in QEMU
test: test-x86_64

test-x86_64:
	$(CLI_BIN) test --arch x86_64

test-aarch64:
	$(CLI_BIN) test --arch aarch64

# Create ISO images
iso: iso-x86_64 iso-aarch64

iso-x86_64:
	$(CLI_BIN) iso --arch x86-64

iso-aarch64:
	$(CLI_BIN) iso --arch aarch64

# Create UEFI disk images
disk: disk-x86_64 disk-aarch64

disk-x86_64:
	$(CLI_BIN) disk --arch x86-64

disk-aarch64:
	$(CLI_BIN) disk --arch aarch64

# Clean all build artifacts
clean:
	$(CLI_BIN) clean --all

# Show project info
info: cli
	$(CLI_BIN) info --verbose

# Show help
help:
	@echo "MAN Linux Distribution — Build Commands"
	@echo ""
	@echo "  make setup           — install deps & clone Toolchain"
	@echo "  make build           — build both x86_64 & aarch64"
	@echo "  make build-x86_64    — build x86_64 only"
	@echo "  make build-aarch64   — build aarch64 only"
	@echo "  make release         — build and package ISOs for both architectures"
	@echo "  make cosmic-aarch64  — build COSMIC and regenerate the ARM64 image"
	@echo "  make cosmic-x86_64   — build COSMIC and regenerate the x86_64 image"
	@echo "  make test            — boot x86_64 in QEMU"
	@echo "  make test-aarch64    — boot aarch64 in QEMU"
	@echo "  make iso             — create ISO images for both arches"
	@echo "  make disk            — create UEFI disk images for both arches"
	@echo "  make disk-aarch64    — create UEFI disk image for aarch64"
	@echo "  make clean           — clean all build artifacts"
	@echo "  make info            — show project info"
	@echo ""
	@echo "Or use the CLI directly:"
	@echo "  ./man/target/release/man <command> [options]"
	@echo ""

# Check that the CLI binary exists
check: cli
	@test -f $(CLI_BIN) && echo "✓ MAN CLI is built at $(CLI_BIN)" || (echo "✗ MAN CLI not found — run 'make cli'" && exit 1)
