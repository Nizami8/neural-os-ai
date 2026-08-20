#!/usr/bin/env bash
# Cloud Agent install phase for the Neural OS (RISC-V) codebase.
# Idempotent: safe to run repeatedly and while building an environment snapshot.
set -euo pipefail

echo "==> Installing RISC-V GNU toolchains, cross libc, and QEMU"
# - gcc/binutils-riscv64-unknown-elf : bare-metal target used by build.sh (as/ld/objcopy/size)
# - gcc-riscv64-linux-gnu + libc6-dev-riscv64-cross : Linux userspace target used by build-milkv.sh
# - qemu-system-misc (qemu-system-riscv64) + qemu-user (qemu-riscv64) : run/emulate RISC-V builds
PKGS=(
  gcc-riscv64-unknown-elf
  binutils-riscv64-unknown-elf
  gcc-riscv64-linux-gnu
  libc6-dev-riscv64-cross
  qemu-system-misc
  qemu-user
)

# Retry to tolerate transient archive/mirror 400 errors seen from the caching proxy.
apt_ok=0
for i in 1 2 3 4 5; do
  sudo apt-get update -qq || true
  if sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends --fix-missing "${PKGS[@]}"; then
    apt_ok=1
    break
  fi
  echo "   apt install attempt ${i} failed; retrying in $((i * 4))s..."
  sleep $((i * 4))
done
if [ "${apt_ok}" -ne 1 ]; then
  echo "ERROR: failed to install system packages after retries" >&2
  exit 1
fi

echo "==> Installing Rust nightly (needed for -Z build-std) with rust-src"
# Overlay/container filesystems can fail rustup's rename during channel upgrades
# (EXDEV / "Invalid cross-device link"). Keep a working nightly when upgrade fails.
if ! rustup toolchain install nightly --profile minimal --component rust-src; then
  if rustc +nightly -vV >/dev/null 2>&1; then
    echo "WARN: nightly upgrade failed; keeping existing nightly toolchain"
    rustup component add --toolchain nightly rust-src || true
  else
    echo "ERROR: failed to install Rust nightly" >&2
    exit 1
  fi
fi

echo "==> Adding RISC-V targets to the default toolchain and nightly"
# Default toolchain: used by build.sh's bare `rustc`.
rustup target add riscv64gc-unknown-none-elf riscv64gc-unknown-linux-gnu
# Nightly: used by build-milkv.sh (`-Z build-std`).
rustup target add --toolchain nightly riscv64gc-unknown-none-elf riscv64gc-unknown-linux-gnu || true
if ! rustup +nightly target list --installed 2>/dev/null | grep -q 'riscv64gc-unknown-none-elf'; then
  echo "ERROR: nightly missing riscv64gc-unknown-none-elf target" >&2
  exit 1
fi

echo "==> Providing the linker name expected by .cargo/config.toml"
# .cargo/config.toml sets linker = "riscv64-unknown-linux-gnu-gcc";
# Ubuntu ships it as "riscv64-linux-gnu-gcc". Bridge the two names.
if [ ! -e /usr/local/bin/riscv64-unknown-linux-gnu-gcc ]; then
  sudo ln -sf "$(command -v riscv64-linux-gnu-gcc)" /usr/local/bin/riscv64-unknown-linux-gnu-gcc
fi

echo "==> Toolchain versions"
riscv64-unknown-elf-gcc --version | head -1
riscv64-linux-gnu-gcc --version | head -1
qemu-system-riscv64 --version | head -1
qemu-riscv64 --version | head -1
rustc +nightly --version

echo "==> Neural OS RISC-V dev environment ready."
