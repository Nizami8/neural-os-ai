# Neural OS v1.0-alpha — Stage 4

Self-learning RISC-V microkernel: TrapFrame scheduling, processes/threads, L4-style IPC, and capability checks.

ChatGPT Stage 1–3 archives never landed in this repository (boot, trap, scheduler, syscalls were referenced but missing). This tree is a complete, buildable Stage 1–4 kernel plus the existing neural observer.

## What runs

| Stage | Contents |
|-------|----------|
| 1 | M-mode boot, BSS clear, UART, `TrapFrame`, trap entry |
| 2 | CLINT timer, preemptive round-robin, per-thread stacks |
| 3 | `Process` / `Thread`, syscall ABI (`yield`, `getpid`, `gettid`, `print`, `exit`) |
| 4 | Endpoint rendezvous, blocking `send`/`recv`, capability rights + generation |

IPC is capability → endpoint → waiting sender/receiver. There is no `send(pid)`. A send that finds a waiting receiver completes immediately (short IPC). Otherwise the sender blocks until a matching recv.

## Build & run

```bash
chmod +x build.sh run.sh
./build.sh
./run.sh          # qemu-system-riscv64 -machine virt -bios none
```

Host tests (no QEMU required):

```bash
cargo test
```

Milk-V / Linux userspace simulation:

```bash
cargo run --features std --bin milkv-userspace
```

## Expected QEMU output

```
Neural OS v1.0-alpha  Stage 4: IPC + Capabilities
[server] recv on cap 1
[client] send on cap 1
[server] got 42 from <pid> seq=0
[denied] send status=... (expect denied)
```

## Layout

| Path | Role |
|------|------|
| `src/trapframe.rs` | Full GPR + CSR frame (`mret` restore) |
| `src/context.rs` / `context.S` | Callee-saved kernel switch |
| `src/thread.rs` / `process.rs` | TCB and process + cap table |
| `src/scheduler.rs` | Round-robin; AI may retune priority only |
| `src/syscall.rs` | Dispatcher |
| `src/ipc.rs` | Endpoint wait queues |
| `src/capability.rs` | Rights, generation, expiry |
| `src/kernel.rs` | Object graph + rendezvous |
| `src/neural.rs` / `adaptive.rs` | MLP + fairness observer (not in the switch path) |

## Syscalls

| # | Name | Notes |
|---|------|--------|
| 0 | `yield` | Reschedule |
| 1 | `getpid` | |
| 2 | `gettid` | |
| 3 | `print` | Kernel-identity pointer + len |
| 4 | `exit` | Marks thread exited |
| 5 | `send` | `a0=cap, a1..a3=words` |
| 6 | `recv` | Blocks or rendezvous; returns words in `a1..a3`, sender in `a6` |
| 7 | `create_endpoint` | Inserts a cap into the caller |
| 8 | `grant` | Subset of rights to another process |

## Architecture

```
ecall / timer IRQ
        │
        ▼
     trap.S  (save TrapFrame via mscratch)
        │
        ▼
  rust_trap_handler
        │
   ┌────┴─────┐
   ▼          ▼
 syscall    timer
   │          │
   ▼          ▼
 send/recv  preempt
   │          │
   └────┬─────┘
        ▼
    scheduler
        │
        ▼
  trap_return / mret
```
