//! Host-testable view of the Neural OS logic modules.
//!
//! The kernel itself is `#![no_std]` and built from `src/main.rs` via
//! `./build.sh`. This library target exists purely so the pure-computation
//! modules can be unit-tested on the host with `cargo test`.
#![cfg_attr(not(test), no_std)]

pub mod neural;
pub mod storage;
