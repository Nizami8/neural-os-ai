#!/bin/bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/common.sh"

banner "🚀 Deploying Neural OS to Milk-V Duo"

MILKV_IP="${1:-$MILKV_DEFAULT_IP}"

require_file "$MILKV_BINARY" "Run: ./build-milkv.sh first"

echo ""
echo "1️⃣  Finding Milk-V Duo IP address..."

# Попытаемся ping'ануть
if ! ping -c 1 "$MILKV_IP" &> /dev/null; then
    warn "Could not reach $MILKV_IP"
    step "Trying to find via nmap..."

    if have_cmd nmap; then
        nmap -sn 192.168.1.0/24 | grep -i "milkv\\|duo\\|sg2042" || true
    fi

    echo "   Manually specify IP: ./deploy-milkv.sh <IP>"
    read -p "   Enter IP: " MILKV_IP
fi

echo "   ✓ Target: $MILKV_IP"

echo ""
echo "2️⃣  Checking SSH connection..."

if ! ssh -q -o ConnectTimeout=2 "$MILKV_USER@$MILKV_IP" "echo OK" &>/dev/null; then
    die "Cannot SSH to $MILKV_USER@$MILKV_IP" "Maybe: ssh-keygen -R $MILKV_IP"
fi

echo "   ✓ SSH connected"

echo ""
echo "3️⃣  Uploading binary..."

run scp -q "$MILKV_BINARY" "$MILKV_USER@$MILKV_IP:$MILKV_REMOTE_PATH"
run ssh "$MILKV_USER@$MILKV_IP" "chmod +x $MILKV_REMOTE_PATH"

echo "   ✓ Binary uploaded"

echo ""
echo "4️⃣  Running Neural OS..."
echo ""
separator

ssh "$MILKV_USER@$MILKV_IP" "$MILKV_REMOTE_PATH"

echo ""
separator
ok "Done!"
