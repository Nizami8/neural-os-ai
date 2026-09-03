#!/bin/bash
set -euo pipefail

echo "🔨 Building Neural OS for Milk-V Duo 256M"
echo "========================================"

# Проверяем toolchain
if ! command -v riscv64-unknown-linux-gnu-gcc &> /dev/null; then
    echo "❌ Error: riscv64-unknown-linux-gnu-gcc not found"
    echo "   Run: source ~/.milkv-env"
    exit 1
fi

# Проверяем Rust target
if ! rustup target list | grep -q "riscv64gc-unknown-linux-gnu (installed)"; then
    echo "📦 Installing Rust target..."
    rustup target add riscv64gc-unknown-linux-gnu
fi

echo ""
echo "  → Compiling Neural OS (userspace binary for Linux)"
echo ""

cargo build \
    --target riscv64gc-unknown-linux-gnu \
    --release \
    --bin milkv-userspace \
    -Z build-std=core,alloc

if [ ! -f "target/riscv64gc-unknown-linux-gnu/release/milkv-userspace" ]; then
    echo "❌ Build failed!"
    exit 1
fi

BINARY="target/riscv64gc-unknown-linux-gnu/release/milkv-userspace"
SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY")
SIZE_KB=$((SIZE / 1024))

echo ""
echo "✅ Build successful!"
echo ""
echo "📊 Binary Information:"
echo "   File: $BINARY"
echo "   Size: ${SIZE_KB} KB"
echo ""
echo "📋 Ready to deploy:"
echo "   1. scp $BINARY root@192.168.1.100:/root/neural-os"
echo "   2. ssh root@192.168.1.100"
echo "   3. /root/neural-os"
echo ""
