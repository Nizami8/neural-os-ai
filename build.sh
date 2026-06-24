#!/bin/bash

echo "🔨 Building Neural OS v0.8 with AI Learning..."

# Create output directory
mkdir -p build

# Compile assembly files
echo "  → Assembling start.s"
riscv64-unknown-elf-as start.s -o build/start.o || exit 1

echo "  → Assembling trap.s"
riscv64-unknown-elf-as trap.s -o build/trap.o || exit 1

echo "  → Assembling context_switch.s"
riscv64-unknown-elf-as context_switch.s -o build/context_switch.o || exit 1

# Compile Rust to object file
echo "  → Compiling Rust (main.rs + AI modules)"
rustc --target riscv64gc-unknown-none-elf \
    -C panic=abort \
    -C opt-level=z \
    -A warnings \
    --crate-type=lib \
    --emit=obj \
    src/main.rs \
    -o build/main.o || exit 1

if [ ! -f "build/main.o" ]; then
    echo "❌ Failed to compile main.rs"
    exit 1
fi

# Link everything
echo "  → Linking kernel"
riscv64-unknown-elf-ld -T linker.ld \
    build/start.o \
    build/trap.o \
    build/context_switch.o \
    build/main.o \
    -o build/kernel.elf || exit 1

if [ ! -f "build/kernel.elf" ]; then
    echo "❌ Failed to link kernel"
    exit 1
fi

# Create binary
echo "  → Creating binary"
riscv64-unknown-elf-objcopy -O binary build/kernel.elf build/os.bin || exit 1

# Statistics
echo ""
echo "✅ Build successful!"
echo ""
echo "Binary size:"
ls -lh build/os.bin
echo ""
echo "Kernel sections:"
riscv64-unknown-elf-size build/kernel.elf
echo ""
echo "📊 Features compiled:"
echo "   ✓ Preemptive Multitasking"
echo "   ✓ Neural Network Scheduler"
echo "   ✓ Online Learning (SGD)"
echo "   ✓ Metrics Collection"
