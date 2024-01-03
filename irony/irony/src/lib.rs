#![feature(macro_metavar_expr)]

mod common;
mod constraint;
mod entity;
mod environ;
mod operation;
mod pass;
mod printer;
mod region;

mod hash;

pub mod utils;

pub use common::*;
pub use constraint::*;
pub use entity::*;
pub use environ::*;
pub use hash::*;
pub use operation::*;
pub use pass::*;
pub use printer::*;
pub use region::*;

pub mod preclude {
  pub use std::cell::{RefCell, RefMut};
  pub use std::hash::Hash;
  pub use std::ops::DerefMut;
  pub use std::rc::Rc;

  pub use super::*;
}

pub use indexmap;
pub use visible::StructFields;
