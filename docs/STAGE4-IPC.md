# Stage 4 — IPC, endpoints, capabilities

This is the stage ChatGPT failed to emit after Stages 1–3. The model is L4-style **rendezvous**, not a message queue in the kernel fast path.

## Send

1. Look up `cap` in the current process table.
2. Require `ObjectType::Endpoint`, `RIGHT_SEND`, matching **generation**, not expired.
3. If a receiver is waiting: copy the 3-word message, wake the receiver, return immediately.
4. Otherwise enqueue the sender, set `BlockedSend`, reschedule.

## Recv

Same checks with `RIGHT_RECV`. A waiting sender completes the rendezvous; otherwise the receiver blocks.

## Capabilities

A capability is not a PID:

- object type and id
- generation (bumped on endpoint reuse)
- rights (`SEND`, `RECV`, `GRANT`, `MAP`, `DELETE`)
- owner
- optional expiry tick

`SYS_GRANT` requires `RIGHT_GRANT` and inserts a rights-restricted copy into the destination process table.

## Why this is Stage 4

Stages 1–3 give a trapframe, preemption, and syscalls. Without blocking IPC and cap checks, processes cannot wait on each other safely. Stage 5 (not in this change) is the AI scheduler as a **policy observer** that retunes `priority` / quantum — it does not pick the next thread directly.
