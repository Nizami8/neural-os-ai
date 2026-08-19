#!/bin/bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/common.sh"

echo "🔨 Building Neural OS v0.9 - Full AI Orchestration Kernel..."

mkdir -p build

step "Assembling architecture files"
for asm in start trap context_switch; do
    run riscv64-unknown-elf-as "$asm.s" -o "build/$asm.o"
done

step "Compiling Rust + AI modules"
run rustc --target riscv64gc-unknown-none-elf \
    -C panic=abort \
    -C opt-level=z \
    -A warnings \
    --crate-type=lib \
    --emit=obj \
    src/main.rs \
    -o build/main.o

require_file build/main.o "Rust compilation failed"

step "Linking final kernel"
run riscv64-unknown-elf-ld -T linker.ld \
    build/start.o \
    build/trap.o \
    build/context_switch.o \
    build/main.o \
    -o build/kernel.elf

require_file build/kernel.elf "Linking failed"

step "Creating binary"
run riscv64-unknown-elf-objcopy -O binary build/kernel.elf build/os.bin

echo ""
ok "Build successful!"
echo ""
echo "📦 Kernel Statistics:"
riscv64-unknown-elf-size build/kernel.elf
echo ""
echo "Binary size:"
ls -lh build/os.bin
echo ""
echo "📊 Features compiled:"
echo "   ✓ Multi-layer Neural Network (7→8→1)"
echo "   ✓ Momentum-based SGD"
echo "   ✓ Load Balancing with Fairness"
echo "   ✓ Predictive Preemption"
echo "   ✓ Real-time Priority Support"
echo "   ✓ Q-Learning Reinforcement"
echo "   ✓ Metrics Collection"
echo "   ✓ Persistent Weight Storage"
echo "   ✓ Live Statistics Monitor"
