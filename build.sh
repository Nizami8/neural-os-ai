#!/bin/bash
set -euo pipefail

echo "🔨 Building Neural OS v0.9 - Full AI Orchestration Kernel (bare-metal RISC-V)"

BUILD=build
mkdir -p "$BUILD"

AS=riscv64-unknown-elf-as
LD=riscv64-unknown-elf-ld
OBJCOPY=riscv64-unknown-elf-objcopy
SIZE=riscv64-unknown-elf-size

echo "  → Assembling boot + trap vector"
$AS -march=rv64gc -mabi=lp64d start.s -o "$BUILD/start.o"
$AS -march=rv64gc -mabi=lp64d trap.s  -o "$BUILD/trap.o"

echo "  → Compiling + linking Rust kernel (nightly, riscv64gc-unknown-none-elf)"
# Let rustc drive the link so libcore / compiler_builtins are pulled in, but use
# the RISC-V GNU linker with our linker script and the assembled boot objects.
rustc +nightly --target riscv64gc-unknown-none-elf \
    -C panic=abort \
    -C opt-level=z \
    -A warnings \
    -C linker="$LD" \
    -C linker-flavor=ld \
    -C link-arg=-Tlinker.ld \
    -C link-arg="$BUILD/start.o" \
    -C link-arg="$BUILD/trap.o" \
    --crate-type=bin \
    -o "$BUILD/kernel.elf" \
    src/main.rs

echo "  → Creating binary"
$OBJCOPY -O binary "$BUILD/kernel.elf" "$BUILD/os.bin"

echo ""
echo "✅ Build successful!"
echo ""
echo "📦 Kernel Statistics:"
$SIZE "$BUILD/kernel.elf"
echo ""
echo "Binary size:"
ls -lh "$BUILD/os.bin"
echo ""
echo "▶️  Run with: ./run.sh"
