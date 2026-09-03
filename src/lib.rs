//! Host-testable view of the Neural OS logic modules.
//!
//! The kernel itself is `#![no_std]` and built from `src/main.rs` via
//! `./build.sh`. This library target exists so pure-computation modules can be
//! unit-tested on the host with `cargo test`, and so the Milk-V userspace
//! binary can share the same neural/storage code.
#![cfg_attr(not(test), no_std)]

pub mod fixed;
pub mod neural;
pub mod storage;
