//! cmtir is the central intermediate representation (IR) for Cement (cmt2) languages and compiler tools.
//!
//! Key features:
//! + Rule-based syntax and semantics
//! + Natural parse-print support
//! + Def-use support for operations
//! + Compatibility with FIRRTL
//!
//! ## Design
//! 
//! cmtir is implemented based on [kir](https://github.com/arch-of-shadow/kir), which provides a consistent pattern of parse-print support for all IR components.
//!
//! `src/ir` defines different components of `cmtir`:
//!
//! + `ir::op`: operations.
//! + `ir::instance`: instance declaration and usage.
//! + `ir::literal`: literals.
//! + `ir::rule`: rules.
//! + `ir::structure`: module and circuit.
//! + `ir::error`: span and error messages.
//!
//! All structs in `cmtir` have the consistent pattern of `SExpr` derive, which is a macro that provides parse-print support (see [kir::SExpr](https://github.com/arch-of-shadow/kir/blob/uv/macros/src/sexpr.rs)).
//!
//! For operations, `cmtir` derives `OpIO` trait (also defined in `kir`), which provides def-use support: taking/updating def/use values of operations. For most operations, the `OpIO` trait is derived automatically when `#[opio(xxx)]` is specified on the struct fields.
//! 
//! ## External Module
//! 
//! `cmtir` accepts **FIRRTL code as external modules**. This is useful for reusing existing modules from the FIRRTL (or Chisel, Chipyard) ecosystem. Also, since FIRRTL supports blackbox SV modules, `cmtir` is also compatible with SV legacy code indirectly. 
//! 
//! The question is, how `cmtir` treat RTL external modules in the rule-based semantics? `cmtir` provides syntax to declare **external rules** for external modules, each of which specifies the real hardware behavior of the corresponding rule, like setting up some signals. `cmtir` also provides syntax to **bind** `cmtir` module ports and RTL ports in external modules.


pub mod common;
pub use common::*;

pub mod utils;
pub use utils::*;

pub use kir::*;

pub mod ir;
pub use ir::*;

pub mod fsmgen;

pub mod to_fir;
pub use to_fir::*;

pub use anyhow;
pub use json;

pub mod fir;
