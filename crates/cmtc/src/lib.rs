//! The cmtc compiler
//!
//! `cmtc` provides passes for `cmtir` to generate  [FIRRTL](https://bar.eecs.berkeley.edu/projects/firrtl.html) (and use [firtool](https://github.com/llvm/circt/releases) to generate optimized SystemVerilog) and different simulation backends (notably [Verilator](https://www.veripool.org/verilator/) and [Khronos](https://github.com/pku-liang/ksim)).
//!
//! `cmtc` also provides driver functions (in `crate::ehdl`) for the eHDL
//! `cmtrs`, generate cmtir, and compile to
//! FIRRTL/SystemVerilog/Verilator/Khronos.
//!
//! ## Passes
//! > placed in `crate::passes`
//!
//! ## FIRRTL backend
//! > placed in `crate::to_fir`
//!
//! ## Simulation backends
//! > placed in `crate::sim`
//!
//! ## VERY IMPORTANT!!!
//!
//! `cmtrs` use nightly features for span support. You need add a file
//! `.cargo/config.toml` with the following content: ```toml
//! [build]
//! rustflags = "--cfg procmacro2_semver_exempt"
//! ```
//! as well as a `rust-toolchain.toml` file with the following content:
//! ```toml
//! [toolchain]
//! channel = "nightly-2024-12-25"
//! ```

pub use cmtir::utils;
use cmtir::*;

mod builtin;

pub mod passes;
pub use passes::*;

pub mod to_fir;
pub use to_fir::*;

pub mod sim;
pub use sim::*;

pub mod ehdl;
pub use ehdl::*;

#[cfg(test)]
mod unit_tests;
