#!/bin/bash
set -euo pipefail

fail() {
    echo "❌ $1"
    exit 1
}

echo "🚀 Milk-V Duo 256M Setup Script for Neural OS"
echo "============================================"

# Проверяем ОС
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
else
    echo "❌ Неподдерживаемая ОС. Нужен Linux или macOS"
    exit 1
fi

echo "📦 Step 1: Installing system dependencies..."

if [ "$OS" = "linux" ]; then
    # Ubuntu/Debian
    sudo apt-get update || fail "apt-get update failed"
    sudo apt-get install -y \
        git build-essential python3 python3-dev python3-pip \
        bc u-boot-tools flex bison wget cpio device-tree-compiler \
        dosfstools mtools parted \
        libssl-dev libncurses-dev riscv64-unknown-elf-gcc \
        || fail "apt-get install failed"

elif [ "$OS" = "macos" ]; then
    # macOS
    brew install git python3 wget dtc mtools parted || fail "brew install failed"
fi

echo "✅ System dependencies installed"

echo ""
echo "📥 Step 2: Downloading Milk-V Duo Buildroot SDK..."

# Скачиваем SDK
mkdir -p ~/milkv-workspace
cd ~/milkv-workspace || fail "Cannot enter ~/milkv-workspace"

if [ ! -d "duo-buildroot-sdk" ]; then
    echo "  → Cloning buildroot SDK..."
    git clone https://github.com/milkv-duo/duo-buildroot-sdk.git || fail "Cloning buildroot SDK failed"
    cd duo-buildroot-sdk || fail "Cannot enter duo-buildroot-sdk"
else
    echo "  → SDK already exists, updating..."
    cd duo-buildroot-sdk || fail "Cannot enter duo-buildroot-sdk"
    git pull origin master || fail "Updating buildroot SDK failed"
fi

echo "✅ SDK ready at ~/milkv-workspace/duo-buildroot-sdk"

echo ""
echo "🔧 Step 3: Installing Rust & RISC-V toolchain..."

# Установляем Rust
if ! command -v rustc &> /dev/null; then
    echo "  → Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        || fail "Rust installation failed"
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env" || fail "Cannot load Rust environment from $HOME/.cargo/env"
else
    echo "  → Rust already installed"
fi

# Добавляем RISC-V target
rustup target add riscv64gc-unknown-linux-gnu || fail "Cannot add target riscv64gc-unknown-linux-gnu"

# Проверяем GCC
if ! command -v riscv64-unknown-elf-gcc &> /dev/null; then
    echo "  → Downloading RISC-V GCC toolchain..."
    
    # Скачиваем prebuilt toolchain
    if [ "$OS" = "linux" ]; then
        TOOLCHAIN_URL="https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2024.06.28/riscv64-glibc-ubuntu-22.04-gcc-nightly-2024.06.28-nightly.tar.gz"
        TOOLCHAIN_FILE="riscv-toolchain-linux.tar.gz"
    else
        TOOLCHAIN_URL="https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2024.06.28/riscv64-glibc-macos-12-gcc-nightly-2024.06.28-nightly.tar.gz"
        TOOLCHAIN_FILE="riscv-toolchain-macos.tar.gz"
    fi
    
    cd ~/milkv-workspace || fail "Cannot enter ~/milkv-workspace"
    wget -O "$TOOLCHAIN_FILE" "$TOOLCHAIN_URL" || fail "Downloading RISC-V toolchain failed"
    tar xzf "$TOOLCHAIN_FILE" || fail "Unpacking $TOOLCHAIN_FILE failed"
    rm "$TOOLCHAIN_FILE"
fi

echo "✅ Rust & RISC-V toolchain ready"

echo ""
echo "📋 Step 4: Setting up environment variables..."

# Создаем файл конфигурации
cat > ~/.milkv-env << 'EOF' || fail "Cannot write ~/.milkv-env"
#!/bin/bash

# Milk-V Duo Development Environment
export MILKV_SDK="$HOME/milkv-workspace/duo-buildroot-sdk"
export RISCV_PATH="$HOME/milkv-workspace/riscv64-glibc-ubuntu-22.04-gcc-nightly-2024.06.28-nightly"

# Если используешь собственный toolchain
if [ -d "$RISCV_PATH" ]; then
    export PATH="$RISCV_PATH/bin:$PATH"
fi

# Rust
source "$HOME/.cargo/env"
export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-unknown-linux-gnu-gcc

echo "✅ Milk-V Duo environment loaded"
EOF

echo "  → Created ~/.milkv-env"
echo "  → Add to ~/.bashrc: source ~/.milkv-env"

echo ""
echo "============================================"
echo "✅ Setup Complete!"
echo ""
echo "Next steps:"
echo "  1. source ~/.milkv-env"
echo "  2. Clone your Neural OS repo"
echo "  3. cargo build --target riscv64gc-unknown-linux-gnu"
echo "  4. Follow flashing guide"
echo ""
