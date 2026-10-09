# Build for Anthropic's OSS Scanner (https://github.com/anthropics/oss-scanner).
# VocaWin ships for Windows only, but the Tauri crate also builds and runs its
# unit tests on Linux with CPU-only whisper.cpp (see AGENTS.md). That Linux
# build is what this image provides. Windows-only code (cfg(windows): WASAPI,
# SendInput injection, low-level hooks, Vulkan, DirectML) is in the checkout
# for source review but is not compiled here.
# trixie: ort's prebuilt ONNX Runtime needs glibc >= 2.38 (__isoc23_strtol).
FROM rust:1-trixie
ENV DEBIAN_FRONTEND=noninteractive \
    CC=clang \
    CXX=clang++ \
    CARGO_TERM_COLOR=never \
    RUSTFLAGS="-C force-frame-pointers=yes"
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates build-essential clang libclang-dev cmake ninja-build pkg-config git python3 \
      libasound2-dev \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev \
      libjavascriptcoregtk-4.1-dev libssl-dev \
      nodejs npm \
    && rm -rf /var/lib/apt/lists/*

# scanner contract: the checkout lives inside the image, at /src
COPY . /src
WORKDIR /src

# Frontend: tauri::generate_context! needs ../dist at compile time.
RUN npm ci --no-audit --no-fund && npm run build

# Fetch every crate (and the ONNX Runtime binaries ort downloads) while the
# network is up, then build the test binaries so `cargo test --offline` works
# in the scan with no Internet.
WORKDIR /src/src-tauri
RUN cargo fetch --locked \
    && cargo test --locked --no-run \
    && cargo build --locked

# Run the unit tests once so we know the image works; failures do not abort.
RUN cargo test --locked --offline || echo "WARNING: cargo test reported failures"

WORKDIR /src
