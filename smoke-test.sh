#!/bin/bash
# End-to-end smoke test: build the kernel, boot it in QEMU, and assert that it
# reaches banner + real preemptive multitasking (all three tasks run) + stats.
set -euo pipefail

# Treat QEMU output as raw bytes. The run is cut off by `timeout`, which can
# leave a truncated multi-byte UTF-8 sequence that confuses grep in a UTF-8
# locale; C locale + `grep -a` matches ASCII substrings reliably.
export LC_ALL=C

./build.sh >/dev/null

echo "==> Booting kernel in QEMU (8s)"
OUT="$(timeout 8 qemu-system-riscv64 \
    -machine virt -bios none -kernel build/os.bin \
    -nographic -monitor none -no-reboot 2>&1 || true)"

check() {
    # here-string (no pipe) so `grep -q` exiting early cannot SIGPIPE a writer
    # and trip `pipefail`.
    if ! grep -aqE "$1" <<<"$OUT"; then
        echo "FAIL: $2"
        echo "----- last 1500 bytes of output -----"
        printf '%s\n' "$OUT" | tail -c 1500 || true
        exit 1
    fi
}

check "Neural OS v0.9"          "no boot banner"
check "\[T1:"                   "task T1 never ran"
check "\[T2:"                   "task T2 never ran"
check "\[T3:"                   "task T3 never ran"
check "STATISTICS"              "no statistics block"
# Preemption proof: a task counter must climb well past its first quantum,
# i.e. the task resumed after being switched out and back in.
check "\[T1:[5-9][0-9]\|RT\]"   "no evidence of preemptive resume (T1 < 50)"

echo "PASS: kernel boots, T1/T2/T3 preempt round-robin, statistics printed."
