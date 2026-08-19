#!/bin/bash
# Shared configuration and helpers for the Neural OS build/deploy scripts.
# Source it from a script at the repository root:
#     source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/common.sh"

MILKV_TARGET="riscv64gc-unknown-linux-gnu"
MILKV_BINARY="target/$MILKV_TARGET/release/milkv-userspace"
MILKV_DEFAULT_IP="192.168.1.100"
MILKV_USER="root"
MILKV_REMOTE_PATH="/root/neural-os"
SEPARATOR="========================================"

# banner <title> — title followed by a separator line.
banner() {
    echo "$1"
    echo "$SEPARATOR"
}

separator() {
    echo "$SEPARATOR"
}

step() {
    echo "  → $1"
}

ok() {
    echo "✅ $1"
}

warn() {
    echo "⚠️  $1"
}

# die <message> [hint ...] — report an error with indented hints and exit 1.
die() {
    local message="$1"
    shift
    echo "❌ $message" >&2
    local hint
    for hint in "$@"; do
        echo "   $hint" >&2
    done
    exit 1
}

# run <command...> — run a command, aborting with an error if it fails.
run() {
    "$@" || die "Command failed: $*"
}

have_cmd() {
    command -v "$1" &> /dev/null
}

# require_cmd <command> [hint ...] — abort unless the command is on PATH.
require_cmd() {
    local cmd="$1"
    shift
    have_cmd "$cmd" || die "Error: $cmd not found" "$@"
}

# require_file <path> [hint ...] — abort unless the file exists.
require_file() {
    local path="$1"
    shift
    [ -f "$path" ] || die "File not found: $path" "$@"
}

# file_size_kb <path> — file size in KiB, portable across GNU/BSD stat.
file_size_kb() {
    local size
    size=$(stat -f%z "$1" 2>/dev/null || stat -c%s "$1")
    echo $((size / 1024))
}

# detect_os — echo "linux" or "macos", non-zero exit on anything else.
detect_os() {
    case "$OSTYPE" in
        linux-gnu*) echo "linux" ;;
        darwin*) echo "macos" ;;
        *) return 1 ;;
    esac
}
