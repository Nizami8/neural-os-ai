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
    if ! rustup target add riscv64gc-unknown-linux-gnu; then
        echo "❌ Failed to install Rust target riscv64gc-unknown-linux-gnu"
        exit 1
    fi
fi

echo ""
echo "  → Compiling Neural OS (userspace binary for Linux)"
echo ""

BINARY="target/riscv64gc-unknown-linux-gnu/release/milkv-userspace"

# Старый бинарник прошел бы проверку ниже даже при сломанной сборке
rm -f "$BINARY"

if ! cargo build \
    --target riscv64gc-unknown-linux-gnu \
    --release \
    --bin milkv-userspace \
    -Z build-std=core,alloc; then
    echo "❌ Build failed (cargo returned a non-zero status)"
    exit 1
fi

if [ ! -f "$BINARY" ]; then
    echo "❌ Build reported success but $BINARY is missing"
    exit 1
fi
if ! SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY"); then
    echo "❌ Could not determine size of $BINARY"
    exit 1
fi
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
