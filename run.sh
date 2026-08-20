#!/usr/bin/env bash
set -euo pipefail

ELF="${1:-build/kernel.elf}"
if [[ ! -f "$ELF" ]]; then
    ELF="target/riscv64gc-unknown-none-elf/release/kernel"
fi
if [[ ! -f "$ELF" ]]; then
    echo "kernel ELF not found; run ./build.sh first"
    exit 1
fi

if ! command -v qemu-system-riscv64 >/dev/null 2>&1; then
    echo "qemu-system-riscv64 is required"
    exit 1
fi

exec qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -bios none \
    -nographic \
    -serial mon:stdio \
    -kernel "$ELF"
