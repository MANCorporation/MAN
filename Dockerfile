# MAN Linux Distribution — Build Docker Image
#
# This Dockerfile provides a reproducible build environment for
# compiling the MAN Linux distribution via Toolchain.
#
# Usage:
#   docker build --build-arg TARGET_ARCH=x86_64 -t man-builder .
#   docker run --rm -v $(pwd)/output:/MAN/output man-builder make O=/MAN/output/x86_64 man-x86_64_defconfig && make O=/MAN/output/x86_64

FROM ubuntu:24.04

LABEL org.opencontainers.image.title="MAN Linux Distribution Builder" \
      org.opencontainers.image.description="Build environment for the MAN Linux distribution" \
      org.opencontainers.image.source="https://github.com/kristihack/MAN"

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install build dependencies for Toolchain + QEMU headless + Rust CLI
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        gcc \
        g++ \
        make \
        cmake \
        git \
        wget \
        curl \
        cpio \
        unzip \
        bc \
        rsync \
        libncurses-dev \
        libssl-dev \
        libelf-dev \
        libffi-dev \
        libcap-dev \
        libattr1-dev \
        python3 \
        python3-pip \
        python3-setuptools \
        gperf \
        flex \
        bison \
        ninja-build \
        mtools \
        dosfstools \
        sed \
        unzip \
        rsync \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain (for the man CLI orchestrator)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
ENV CARGO_HOME=/root/.cargo
ENV RUSTUP_HOME=/root/.rustup

# Create MAN project directory
WORKDIR /MAN

# Copy the Rust CLI source
COPY man/ /MAN/man/

# Build the CLI
RUN cd /MAN/man && cargo build --release

# Make the CLI accessible from anywhere
RUN ln -s /MAN/man/target/release/man /usr/local/bin/man

# Copy Toolchain configs, overlay, and scripts
COPY configs/ /MAN/configs/
COPY overlay/ /MAN/overlay/
COPY scripts/ /MAN/scripts/
COPY Makefile /MAN/Makefile
COPY man.toml /MAN/man.toml

# Build args
ARG TARGET_ARCH=x86_64

# Clone Toolchain at the pinned version
ARG BR_VERSION=2024.08
RUN git clone --depth 1 --branch ${BR_VERSION} https://github.com/buildroot/buildroot.git /MAN/toolchain

# Copy defconfigs into Toolchain
RUN cp /MAN/configs/man-x86_64_defconfig /MAN/toolchain/configs/ \
    && cp /MAN/configs/man-aarch64_defconfig /MAN/toolchain/configs/

# Default command: show help
CMD ["man", "--help"]
