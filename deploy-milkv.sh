#!/bin/bash

echo "🚀 Deploying Neural OS to Milk-V Duo"
echo "===================================="

# Параметры
MILKV_IP="${1:-192.168.1.100}"
MILKV_USER="root"
BINARY="target/riscv64gc-unknown-linux-gnu/release/milkv-userspace"

if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found: $BINARY"
    echo "   Run: ./build-milkv.sh first"
    exit 1
fi

echo ""
echo "1️⃣  Finding Milk-V Duo IP address..."

# Попытаемся ping'ануть
if ! ping -c 1 "$MILKV_IP" &> /dev/null; then
    echo "⚠️  Could not reach $MILKV_IP"
    echo "   Trying to find via nmap..."
    
    if command -v nmap &> /dev/null; then
        nmap -sn 192.168.1.0/24 | grep -i "milkv\\|duo\\|sg2042" || true
    fi
    
    echo "   Manually specify IP: ./deploy-milkv.sh <IP>"
    read -p "   Enter IP: " MILKV_IP
fi

echo "   ✓ Target: $MILKV_IP"

echo ""
echo "2️⃣  Checking SSH connection..."

if ! ssh -q -o ConnectTimeout=2 "$MILKV_USER@$MILKV_IP" "echo OK" &>/dev/null; then
    echo "❌ Cannot SSH to $MILKV_USER@$MILKV_IP"
    echo "   Maybe: ssh-keygen -R $MILKV_IP"
    exit 1
fi

echo "   ✓ SSH connected"

echo ""
echo "3️⃣  Uploading binary..."

scp -q "$BINARY" "$MILKV_USER@$MILKV_IP:/root/neural-os"
chmod +x /tmp/neural-os

echo "   ✓ Binary uploaded"

echo ""
echo "4️⃣  Running Neural OS..."
echo ""
echo "════════════════════════════════════════"

ssh "$MILKV_USER@$MILKV_IP" "/root/neural-os"

echo ""
echo "════════════════════════════════════════"
echo "✅ Done!"
