#!/bin/bash
# End-to-end smoke test: build the kernel, boot it in QEMU, drive the interactive
# UART shell over stdin, and assert the whole stack works: boot, preemptive
# scheduling, syscalls, IPC (+ capabilities) and the shell commands.
set -euo pipefail

# Treat QEMU output as raw bytes (a truncated multi-byte UTF-8 char at the
# timeout boundary would otherwise confuse grep in a UTF-8 locale).
export LC_ALL=C

./build.sh >/dev/null

echo "==> Booting kernel in QEMU and driving the shell (8s)"
OUT="$(printf 'help\nps\nstats\nnn\n' | timeout 8 qemu-system-riscv64 \
    -machine virt -bios none -kernel build/os.bin \
    -nographic -no-reboot 2>&1 || true)"

check() {
    if ! grep -aqE "$1" <<<"$OUT"; then
        echo "FAIL: $2"
        echo "----- last 1500 bytes of output -----"
        printf '%s\n' "$OUT" | tail -c 1500 || true
        exit 1
    fi
}

check "Neural OS v0.9"                            "no boot banner"
# Stage 3 syscalls + Stage 4 capability enforcement (T4 has no capability).
check "T4 started via ecall, pid=4"               "SYS_GETPID/SYS_PRINT not working"
check "T4 SYS_SEND on ep0 DENIED"                 "IPC capability enforcement not working"
# Interactive shell (original project goal: command-line interaction).
check "neural-os>"                                "shell prompt missing (UART input?)"
check "commands: help, ps, stats, nn"             "shell 'help' not working"
check "PID  STATE"                                "shell 'ps' not working"
# Scheduler + IPC state: the consumer is Blocked waiting on recv, shell Running.
check "6  Blocked"                                "IPC blocking / task state not shown by ps"
check "7  Running"                                "shell task not shown Running by ps"
check "Context Switches:"                         "shell 'stats' not working"
check "Neural output weights:"                    "shell 'nn' not working"

echo "PASS: boot + preemption + syscalls + IPC + capabilities + interactive shell all working."
