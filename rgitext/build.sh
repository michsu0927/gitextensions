#!/bin/bash
set -e

PLATFORM="${BUILD_PLATFORM:-windows}"
echo "=== Building rGitExt for platform: $PLATFORM ==="

npm install

mkdir -p dist

case "$PLATFORM" in
  windows)
    echo ">>> Cross-compiling for Windows (x86_64-pc-windows-msvc)..."
    npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
    cp src-tauri/target/x86_64-pc-windows-msvc/release/rgitext.exe dist/ 2>/dev/null || true
    cp -r src-tauri/target/x86_64-pc-windows-msvc/release/bundle/* dist/ 2>/dev/null || true
    ;;
  linux)
    echo ">>> Building for Linux (native x86_64-unknown-linux-gnu)..."
    npx tauri build
    cp src-tauri/target/release/rgitext dist/ 2>/dev/null || true
    cp -r src-tauri/target/release/bundle/* dist/ 2>/dev/null || true
    ;;
  macos)
    echo ">>> Cross-compiling for macOS (x86_64-apple-darwin + aarch64-apple-darwin)..."
    echo "WARNING: macOS cross-compilation from Linux has limitations."
    echo "  - Code signing and notarization require a real macOS host."
    echo "  - Consider using a macOS CI runner (GitHub Actions macos-latest) for production builds."
    npx tauri build --runner cargo-zigbuild --target x86_64-apple-darwin 2>/dev/null || \
      echo "macOS x86_64 build failed (expected without macOS SDK)"
    npx tauri build --runner cargo-zigbuild --target aarch64-apple-darwin 2>/dev/null || \
      echo "macOS aarch64 build failed (expected without macOS SDK)"
    cp -r src-tauri/target/x86_64-apple-darwin/release/bundle/* dist/ 2>/dev/null || true
    cp -r src-tauri/target/aarch64-apple-darwin/release/bundle/* dist/ 2>/dev/null || true
    ;;
  *)
    echo "ERROR: Unknown BUILD_PLATFORM '$PLATFORM'. Use: windows, linux, or macos"
    exit 1
    ;;
esac

echo "=== Build complete. Output in /app/dist ==="
ls -la dist/ 2>/dev/null || echo "(dist folder is empty)"
