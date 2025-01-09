//! Standard library for Cmtrs

use cmtir::to_fir::cmtir_type_to_firrtl_type;
use cmtir::IRDump;
use itertools::chain;

use crate as cmtrs;
use crate::gen_ir::AST;
use crate::*;

pub mod reg;
pub use reg::*;
pub mod fifo;
pub use fifo::*;
pub mod mem;
pub use mem::*;
pub mod sim;
pub use sim::*;

itfc_declare! {
  /// Data type of the wire.
  param T;
  /// A hardware wire with type T. Use [`stl::wire()`] or [`stl::Wire::new()`] to make one.
  pub struct Wire{
    #[name("in")]
    /// input of wire
    in_: input param T,
    /// output of wire
    out: output param T,
  }
  /// `wire.write(value)`. Write value to the wire. Can use `wire %= value` to replace. (Requires the wire is `mut` in Rust)
  method write(in_);
  /// `value = wire.read()`. Read the value of a wire. Can use `&wire` to replace.
  method read() -> (out);
}

impl Wire {
  /// Make a hardware wire with type T = t
  pub fn new(t: &Type) -> Wire { wire(t) }
}

impl CmtAST for &Wire {
  fn ast(self) -> AST { self.read().ast() }
}

impl<T: CmtAST> std::ops::RemAssign<T> for Wire {
  /// Equal to `wire.write(rhs)`
  fn rem_assign(&mut self, rhs: T) { self.write(rhs); }
}

impl ToPrintItem for Wire {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { self.read().to_print_item(__cmt_gen) }
}

/// Make a hardware wire with type T = t
#[module]
pub fn wire(t: &Type) -> Wire {
  let io = io! {
    T: t
  };
  anno!("synthesis": "true");
  let fir_str = r#"
module Wire_{ctype T}:
  input a: {ftype T}
  output out: {ftype T}
  connect out, a"#;

  let ir_ty: cmtir::Type = t.clone().into();
  let fir_str = fir_str.replace("{ctype T}", &ir_ty.ir_dump());
  let fir_str = fir_str.replace("{ftype T}", &cmtir_type_to_firrtl_type(ir_ty).to_string());

  // let io = _cmt_gen.borrow_mut().as_mut().unwrap().io();

  external!(["a".to_string()], ["out".to_string()], None, None, fir_str.to_string(),);
  let write = ext_method!(
    None; None; true;
    (io.in_) {}
  );

  let read = ext_method!(
    None; None; false;
    () -> (io.out) {}
  );

  method_rel!(write C write);
  schedule!(write, read);
}

#[test]
fn test_wire() {
  use utils::setup_logger;
  setup_logger();
  let wire = wire(&Type::SInt(16));
  println!("{}", wire.to_cmtir().ir_dump());
}

itfc_declare!(
  #[doc = r"A general module interface to be dynamically generated, can add arbitraty ios and methods into it"]
  pub struct GeneratedModule {}
);
