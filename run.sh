#!/bin/bash
set -euo pipefail

echo "🚀 Running Neural OS v0.9 in QEMU (RISC-V virt)"

if [ ! -f "build/os.bin" ]; then
    echo "❌ build/os.bin not found! Run ./build.sh first."
    exit 1
fi

# -bios none: boot our raw kernel directly in M-mode at 0x80000000.
# Ctrl-A X to quit the emulator.
exec qemu-system-riscv64 \
    -machine virt \
    -bios none \
    -kernel build/os.bin \
    -nographic \
    -monitor none \
    -serial mon:stdio \
    -no-reboot
