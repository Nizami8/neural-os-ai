#!/usr/bin/env bash
set -euo pipefail

echo "🔨 Building Neural OS v1.0-alpha (Stage 4 IPC kernel)..."

TARGET=riscv64gc-unknown-none-elf
cargo build --release --target "$TARGET" --bin kernel --features kernel

ELF="target/${TARGET}/release/kernel"
if [[ ! -f "$ELF" ]]; then
    echo "❌ Kernel ELF not produced"
    exit 1
fi

mkdir -p build
cp "$ELF" build/kernel.elf

if command -v llvm-objcopy >/dev/null 2>&1; then
    llvm-objcopy -O binary "$ELF" build/os.bin
elif command -v rust-objcopy >/dev/null 2>&1; then
    rust-objcopy --binary-architecture=riscv64 -O binary "$ELF" build/os.bin
elif command -v riscv64-unknown-elf-objcopy >/dev/null 2>&1; then
    riscv64-unknown-elf-objcopy -O binary "$ELF" build/os.bin
else
    echo "⚠️  objcopy not found; ELF is at $ELF"
fi

echo ""
echo "✅ Build successful"
echo "   ELF: $ELF"
if command -v llvm-size >/dev/null 2>&1; then
    llvm-size "$ELF" || true
elif command -v rust-size >/dev/null 2>&1; then
    rust-size "$ELF" || true
fi
echo ""
echo "Run with: ./run.sh"
