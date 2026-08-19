#!/bin/bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/common.sh"

banner "🔨 Building Neural OS for Milk-V Duo 256M"

require_cmd riscv64-unknown-linux-gnu-gcc "Run: source ~/.milkv-env"

# Проверяем Rust target
if ! rustup target list | grep -q "$MILKV_TARGET (installed)"; then
    echo "📦 Installing Rust target..."
    rustup target add "$MILKV_TARGET"
fi

echo ""
step "Compiling Neural OS (userspace binary for Linux)"
echo ""

cargo build \
    --target "$MILKV_TARGET" \
    --release \
    --bin milkv-userspace \
    -Z build-std=core,alloc

require_file "$MILKV_BINARY" "Build failed!"

SIZE_KB=$(file_size_kb "$MILKV_BINARY")

echo ""
ok "Build successful!"
echo ""
echo "📊 Binary Information:"
echo "   File: $MILKV_BINARY"
echo "   Size: ${SIZE_KB} KB"
echo ""
echo "📋 Ready to deploy:"
echo "   1. scp $MILKV_BINARY $MILKV_USER@$MILKV_DEFAULT_IP:$MILKV_REMOTE_PATH"
echo "   2. ssh $MILKV_USER@$MILKV_DEFAULT_IP"
echo "   3. $MILKV_REMOTE_PATH"
echo ""
