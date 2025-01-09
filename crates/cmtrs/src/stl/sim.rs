//! Standard library for simulation

use super::*;
use crate as cmtrs;

itfc_declare! {
  /// A integer for simulation
  pub struct Integer {
    #[name("in")]
    in_: input Type::Integer,
    out: output Type::Integer,
  }
  /// `value = int.read()`. Read the value of an Integer. Can use `&int` to replace.
  method read() -> (out);
  /// `int.write(value)`. Write value to the Integer. Can use `int %= value` to replace. (Requires the Integer is `mut` in Rust)
  method write(in_);
}

impl Integer {
  /// Make an integer for simulation
  pub fn new() -> Integer { integer() }
}
impl CmtAST for &Integer {
  fn ast(self) -> AST { self.read().ast() }
}
impl<T: CmtAST> std::ops::RemAssign<T> for Integer {
  /// Equal to `int.write(rhs)`
  fn rem_assign(&mut self, rhs: T) { self.write(rhs); }
}
impl ToPrintItem for Integer {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { self.read().to_print_item(__cmt_gen) }
}

/// Make an integer for simulation
#[module]
pub fn integer() -> Integer {
  let io = io! {};
  anno!("is_virtual": "true");

  let read = ext_method! {None;None;()->(io.out){}};
  let write = ext_method! {None;None;(io.in_)->(){}};
  schedule!(read, write);
}

/// Make an integer for simulation
#[track_caller]
pub fn int(val: impl Into<cmtir::RadixIntLit>) -> Var { literal(val.into(), &Type::Integer) }

/// Convert an integer to hardware type
#[track_caller]
pub fn int_as(val: impl CmtAST, ty: &Type) -> Var {
  Var::new(AST::sim_from_int(val.ast(), ty.clone(), Some(extract_span_from_location(Location::caller()))))
}
