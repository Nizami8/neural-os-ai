#!/bin/bash
set -euo pipefail

echo "🚀 Deploying Neural OS to Milk-V Duo"
echo "===================================="

is_valid_target() {
    local target=$1

    if [[ "$target" =~ ^(([0-9]{1,3}\.){3}[0-9]{1,3})$ ]]; then
        local octet
        local octets
        IFS='.' read -r -a octets <<< "$target"
        for octet in "${octets[@]}"; do
            if (( 10#$octet > 255 )); then
                return 1
            fi
        done
        return 0
    fi

    [[ "$target" =~ ^[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?)*$ ]]
}

validate_target() {
    if ! is_valid_target "$1"; then
        echo "❌ Invalid deployment target: '$1' (expected an IPv4 address or hostname)" >&2
        exit 1
    fi
}

# Параметры
MILKV_IP="${1:-192.168.1.100}"
MILKV_USER="root"
BINARY="target/riscv64gc-unknown-linux-gnu/release/milkv-userspace"
validate_target "$MILKV_IP"

if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found: $BINARY"
    echo "   Run: ./build-milkv.sh first"
    exit 1
fi

echo ""
echo "1️⃣  Finding Milk-V Duo IP address..."

# Попытаемся ping'ануть
if ! ping -c 1 -- "$MILKV_IP" &> /dev/null; then
    echo "⚠️  Could not reach $MILKV_IP"
    echo "   Trying to find via nmap..."

    if command -v nmap &> /dev/null; then
        nmap -sn 192.168.1.0/24 | grep -i "milkv\\|duo\\|sg2042" || true
    fi

    echo "   Manually specify IP: ./deploy-milkv.sh <IP>"
    read -r -p "   Enter IP: " MILKV_IP
    validate_target "$MILKV_IP"
fi

echo "   ✓ Target: $MILKV_IP"

echo ""
echo "2️⃣  Checking SSH connection..."

if ! ssh -q -o ConnectTimeout=2 -- "$MILKV_USER@$MILKV_IP" "echo OK" &>/dev/null; then
    echo "❌ Cannot SSH to $MILKV_USER@$MILKV_IP"
    echo "   Only remove the known-host entry with ssh-keygen -R after a deliberate reflash/replacement"
    echo "   An unexpected host-key change may indicate a MITM; verify the device and network first"
    exit 1
fi

echo "   ✓ SSH connected"

echo ""
echo "3️⃣  Uploading binary..."

scp -q -- "$BINARY" "$MILKV_USER@$MILKV_IP:/root/neural-os"
ssh -q -- "$MILKV_USER@$MILKV_IP" "chmod +x /root/neural-os"

echo "   ✓ Binary uploaded"

echo ""
echo "4️⃣  Running Neural OS..."
echo ""
echo "════════════════════════════════════════"

ssh -- "$MILKV_USER@$MILKV_IP" "/root/neural-os"

echo ""
echo "════════════════════════════════════════"
echo "✅ Done!"
