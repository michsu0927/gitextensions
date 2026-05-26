FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# BUILD_PLATFORM: windows (default), linux, macos
ARG BUILD_PLATFORM=windows
ENV BUILD_PLATFORM=${BUILD_PLATFORM}

# Install Tauri system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    file \
    build-essential \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    pkg-config \
    clang \
    lld \
    llvm \
    nsis \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js (v20)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install platform-specific Rust targets and tools
RUN if [ "$BUILD_PLATFORM" = "windows" ]; then \
        rustup target add x86_64-pc-windows-msvc \
        && rustup component add llvm-tools-preview \
        && cargo install cargo-xwin \
        && ln -s $(rustc --print sysroot)/bin/llvm-rc /usr/local/bin/llvm-rc; \
    elif [ "$BUILD_PLATFORM" = "macos" ]; then \
        rustup target add x86_64-apple-darwin aarch64-apple-darwin \
        && cargo install cargo-zigbuild \
        && apt-get update && apt-get install -y ziglang && rm -rf /var/lib/apt/lists/*; \
    fi
# linux: native target is already installed by default

WORKDIR /app

# Copy all source files into the container image
COPY . .
COPY build.sh /app/build.sh
RUN chmod +x /app/build.sh

CMD ["/app/build.sh"]
