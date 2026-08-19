#!/bin/bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/common.sh"

banner "🚀 Milk-V Duo 256M Setup Script for Neural OS"

OS=$(detect_os) || die "Неподдерживаемая ОС. Нужен Linux или macOS"

echo "📦 Step 1: Installing system dependencies..."

if [ "$OS" = "linux" ]; then
    # Ubuntu/Debian
    sudo apt-get update
    sudo apt-get install -y \
        git build-essential python3 python3-dev python3-pip \
        bc u-boot-tools flex bison wget cpio device-tree-compiler \
        dosfstools mtools parted \
        libssl-dev libncurses-dev riscv64-unknown-elf-gcc
        
elif [ "$OS" = "macos" ]; then
    # macOS
    brew install git python3 wget dtc mtools parted
fi

ok "System dependencies installed"

echo ""
echo "📥 Step 2: Downloading Milk-V Duo Buildroot SDK..."

# Скачиваем SDK
mkdir -p ~/milkv-workspace
cd ~/milkv-workspace

if [ ! -d "duo-buildroot-sdk" ]; then
    step "Cloning buildroot SDK..."
    git clone https://github.com/milkv-duo/duo-buildroot-sdk.git
    cd duo-buildroot-sdk
else
    step "SDK already exists, updating..."
    cd duo-buildroot-sdk
    git pull origin master
fi

ok "SDK ready at ~/milkv-workspace/duo-buildroot-sdk"

echo ""
echo "🔧 Step 3: Installing Rust & RISC-V toolchain..."

# Установляем Rust
if ! have_cmd rustc; then
    step "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    step "Rust already installed"
fi

# Добавляем RISC-V target
rustup target add "$MILKV_TARGET"

# Проверяем GCC
if ! have_cmd riscv64-unknown-elf-gcc; then
    step "Downloading RISC-V GCC toolchain..."
    
    # Скачиваем prebuilt toolchain
    if [ "$OS" = "linux" ]; then
        TOOLCHAIN_URL="https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2024.06.28/riscv64-glibc-ubuntu-22.04-gcc-nightly-2024.06.28-nightly.tar.gz"
        TOOLCHAIN_FILE="riscv-toolchain-linux.tar.gz"
    else
        TOOLCHAIN_URL="https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2024.06.28/riscv64-glibc-macos-12-gcc-nightly-2024.06.28-nightly.tar.gz"
        TOOLCHAIN_FILE="riscv-toolchain-macos.tar.gz"
    fi
    
    cd ~/milkv-workspace
    wget -O "$TOOLCHAIN_FILE" "$TOOLCHAIN_URL"
    tar xzf "$TOOLCHAIN_FILE"
    rm "$TOOLCHAIN_FILE"
fi

ok "Rust & RISC-V toolchain ready"

echo ""
echo "📋 Step 4: Setting up environment variables..."

# Создаем файл конфигурации
cat > ~/.milkv-env << 'EOF'
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

step "Created ~/.milkv-env"
step "Add to ~/.bashrc: source ~/.milkv-env"

echo ""
separator
ok "Setup Complete!"
echo ""
echo "Next steps:"
echo "  1. source ~/.milkv-env"
echo "  2. Clone your Neural OS repo"
echo "  3. cargo build --target $MILKV_TARGET"
echo "  4. Follow flashing guide"
echo ""
