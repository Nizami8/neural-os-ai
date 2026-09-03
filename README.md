# Neural OS — Self-Learning RISC-V Microkernel

Bare-metal RISC-V kernel with a neural adaptive scheduler, preemptive multitasking,
syscalls, capability-gated IPC, and an interactive UART shell.

## What works today

| Stage | Feature | Status |
|-------|---------|--------|
| Boot | `start.s` + `linker.ld` + CLINT timer | Done |
| 2 | Preemptive multitasking (TrapFrame + per-task stacks) | Done |
| AI | MLP 7→8→1 + momentum SGD, load balancing, Q-learning | Done |
| 3 | Syscalls via `ecall` (`YIELD`, `GETPID`, `PRINT`, `EXIT`, …) | Done |
| 4 | Synchronous IPC + `CAP_SEND` / `CAP_RECV` | Done |
| Shell | Interactive UART CLI: `help`, `ps`, `stats`, `nn` | Done |
| CI | Host unit tests + clippy + QEMU smoke test | Done |

## Quick start (QEMU)

Requires: RISC-V GNU toolchain (`riscv64-unknown-elf-*`), QEMU (`qemu-system-riscv64`),
Rust nightly with `rust-src`.

```bash
# Cloud Agent / local bootstrap (idempotent)
bash .cursor/install.sh

./build.sh          # bare-metal kernel → build/os.bin
./run.sh            # interactive QEMU session
./smoke-test.sh     # automated end-to-end check
cargo test          # host unit tests for neural + storage
```

Expected smoke result:

```text
PASS: boot + preemption + syscalls + IPC + capabilities + interactive shell all working.
```

### Shell commands (in QEMU)

| Command | Description |
|---------|-------------|
| `help` | List commands |
| `ps` | Task table (PID / state) |
| `stats` | Context switches, wait time, fairness |
| `nn` | Neural output weights |

## Architecture

```text
┌──────────────────────────────────────────┐
│         Scheduling Decision              │
└────────────────────┬─────────────────────┘
                     │
    ┌────────────────┼────────────────┐
    ▼                ▼                ▼
┌─────────┐  ┌──────────────┐  ┌───────────┐
│ Neural  │  │Load Balancer │  │Predictive │
│Network  │  │  (Fairness)  │  │Preemption │
└────┬────┘  └──────┬───────┘  └─────┬─────┘
     │               │                │
     └───────────────┼────────────────┘
                     │
             ┌───────▼────────┐
             │ Priority Score │
             └───────┬────────┘
                     │
        ┌────────────▼────────────┐
        ▼                         ▼
    ┌─────────────────┐   ┌──────────────────┐
    │  Task Selection │   │ Real-time Check  │
    │   (Max Score)   │   │  (Deadline)      │
    └────────┬────────┘   └──────────────────┘
             │
      ┌──────▼────────┐
      │ Context Switch│  ← TrapFrame save/restore (trap.s)
      │  + Learning   │
      └───────┬───────┘
              │
       ┌──────▼──────┐
       │ ecall / IPC │  ← syscall.rs + ipc.rs (capabilities)
       └─────────────┘
```

### Neural scheduler

- **MLP**: Input(7) → Hidden(8, Leaky ReLU) → Output(1, Sigmoid)
- **Optimizer**: SGD + momentum (α=0.01, β=0.9)
- **Task classes**: RealTime / Interactive / Batch
- **Fairness**: Jain’s index + hog penalty
- **Reinforcement**: per-task Q-learning from deadline success/failure

### Syscalls & IPC

| Call | Purpose |
|------|---------|
| `SYS_YIELD` | Cooperative yield |
| `SYS_GETPID` | Current task id |
| `SYS_PRINT` | UART print |
| `SYS_EXIT` | Terminate task |
| `SYS_SEND` / `SYS_RECV` | Rendezvous IPC (capability-checked) |
| `SYS_PS` / `SYS_STATS` / `SYS_NN` | Shell info queries |

## Layout

| Path | Purpose |
|------|---------|
| `src/main.rs` | Kernel entry, demo tasks, UART shell |
| `src/scheduler.rs` | Task table, TrapFrame, stacks |
| `src/trap.rs` / `trap.s` | Timer + ecall trap handling |
| `src/trapframe.rs` | RV64 machine-mode frame |
| `src/syscall.rs` | Syscall numbers + wrappers |
| `src/ipc.rs` | Endpoints + capabilities |
| `src/neural.rs` | MLP + momentum SGD |
| `src/adaptive.rs` | AI scheduler + Q-learning |
| `src/storage.rs` | Weight save/load buffer |
| `start.s` / `linker.ld` | Boot + memory map |
| `build.sh` / `run.sh` / `smoke-test.sh` | Build, QEMU, CI smoke |
| `.cursor/install.sh` | Cloud Agent RISC-V toolchain bootstrap |
| `build-milkv.sh` / `deploy-milkv.sh` | Milk-V Duo userspace path |

## Cloud Agent environment

```json
{
  "install": "bash .cursor/install.sh"
}
```

Installs RISC-V GNU toolchains, QEMU, Rust nightly + `rust-src`, and RISC-V targets.
Idempotent; tolerates rustup overlay/`EXDEV` upgrade failures when a working nightly already exists.

## Milk-V Duo (userspace)

Separate Linux userspace binary path (not the bare-metal kernel):

```bash
./setup-milkv.sh
./build-milkv.sh
./deploy-milkv.sh <board-ip>
```

See `MILKV-DEPLOYMENT.md` and `docs/MILKV-GUIDE.md`. Protect root SSH (password or key) before deploying.

## Next steps

- [ ] Reduce `static mut` kernel globals
- [ ] Fixed-point NN (no FPU dependency)
- [ ] Real flash/EEPROM persistence for `storage.rs`
- [ ] Richer capability model (beyond IPC endpoints)
- [ ] Re-validate Milk-V Duo userspace against current modules
- [ ] Multi-core / energy-aware / deeper RL (research)

## References

1. SGD with Momentum — Rumelhart et al. (1986)
2. Real-time Scheduling — Liu & Layland (1973)
3. Fairness — Jain et al. (1984)
4. Q-Learning — Watkins & Dayan (1992)
