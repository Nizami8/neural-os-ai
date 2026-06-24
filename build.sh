#!/bin/bash

echo "🔨 Building Neural OS v0.9 - Full AI Orchestration Kernel..."

mkdir -p build

echo "  → Assembling architecture files"
riscv64-unknown-elf-as start.s -o build/start.o || exit 1
riscv64-unknown-elf-as trap.s -o build/trap.o || exit 1
riscv64-unknown-elf-as context_switch.s -o build/context_switch.o || exit 1

echo "  → Compiling Rust + AI modules"
rustc --target riscv64gc-unknown-none-elf \
    -C panic=abort \
    -C opt-level=z \
    -A warnings \
    --crate-type=lib \
    --emit=obj \
    src/main.rs \
    -o build/main.o || exit 1

if [ ! -f "build/main.o" ]; then
    echo "❌ Rust compilation failed"
    exit 1
fi

echo "  → Linking final kernel"
riscv64-unknown-elf-ld -T linker.ld \
    build/start.o \
    build/trap.o \
    build/context_switch.o \
    build/main.o \
    -o build/kernel.elf || exit 1

if [ ! -f "build/kernel.elf" ]; then
    echo "❌ Linking failed"
    exit 1
fi

echo "  → Creating binary"
riscv64-unknown-elf-objcopy -O binary build/kernel.elf build/os.bin || exit 1

echo ""
echo "✅ Build successful!"
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
