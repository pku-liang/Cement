//! Standard library for registers

use super::*;
use crate as cmtrs;

itfc_declare! {
  /// Data type of the register.
  param T;
  /// A hardware register with type T. Use [`stl::reg()`] or [`stl::Reg::new()`] to make one.
  pub struct Reg{
    /// input of reg
    #[name("in")]
    in_: input param T,
    /// output of reg
    out: output param T,
  }
  /// `value = reg.read()`. Read the value of a reg. Can use `&reg` to replace.
  method read() -> (out);
  /// `reg.write(value)`. Write value to the reg. Can use `reg %= value` to replace. (Requires the reg is `mut` in Rust)
  method write(in_);
}

impl Reg {
  /// Make a hardware register with type T = t
  pub fn new(t: &Type) -> Reg { reg(t) }
}
impl CmtAST for &Reg {
  #[track_caller]
  fn ast(self) -> AST { self.read().ast() }
}
impl<T: CmtAST> std::ops::RemAssign<T> for Reg {
  /// Equal to `reg.write(rhs)`
  #[track_caller]
  fn rem_assign(&mut self, rhs: T) { self.write(rhs); }
}
impl ToPrintItem for Reg {
  #[track_caller]
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { self.read().to_print_item(__cmt_gen) }
}

/// Make a hardware register with type T = t
#[module]
pub fn reg(t: &Type) -> Reg {
  let io = io! {
    T: t.clone()
  };
  anno!("synthesis": "true");
  let fir_str = r#"
module Reg_{ctype T}:
  input write_en: UInt<1>
  input write_data: {ftype T}
  input clock: Clock
  input reset: UInt<1>
  output read_data: {ftype T}

  reg r: {ftype T}, clock

  connect read_data, r
  when write_en:
    connect r, write_data"#;

  let ir_ty: ir::Type = t.clone().into();
  let fir_str = fir_str.replace("{ctype T}", &ir_ty.ir_dump());
  let fir_str = fir_str.replace("{ftype T}", &cmtir_type_to_firrtl_type(ir_ty).to_string());

  external!(
    ["write_data".to_string()],
    ["read_data".to_string()],
    Some("clock".to_string()),
    Some("reset".to_string()),
    fir_str.to_string(),
  );

  let read = ext_method!(
    None; None; false;
    () -> (io.out) {}
  );

  let write = ext_method!(
    Some("write_en".to_string()); None; true;
    (io.in_) {}
  );

  schedule!(read, write);
}

/// make a hardware register with initial value `init` when reset.
#[module]
pub fn reg_init(t: &Type, init: impl CmtLit) -> Reg {
  let io = io! {T:t};
  anno!("synthesis":"true");
  distinguisher!(&format!("init_{}", init.name()));

  let reg_impl = instance!(reg_init_impl(t));

  named_always! {"reset".to_string();
    () { reg_impl.reset(init.lit(&t)); }
  };
  let read = method!(
    () -> (io.out) {reg_impl.read();}
  );
  let write = method!(
    (io.in_) {reg_impl.write(io.in_);}
  );

  method_rel!(write C write);
  schedule!(read, write);
}

itfc_declare! {
  param T;
  struct RegInitImpl{
    #[name("in")]
    in_: input param T,
    init: input param T,
    out: output param T,
  }
  method reset(init);
  method read() -> (out);
  method write(in_);
}

#[module]
fn reg_init_impl(t: &Type) -> RegInitImpl {
  let io = io! {
    T: t
  };

  let fir_str = r#"
module RegInitImpl_{ctype T}:
  input write_en: UInt<1>
  input write_data: {ftype T}
  input clock: Clock
  input reset: UInt<1>
  input reset_value: {ftype T}
  output read_data: {ftype T}

  reg r: {ftype T}, clock

  connect read_data, r
  when write_en:
    connect r, write_data
  when reset:
    connect r, reset_value
  "#;

  let ir_ty: ir::Type = t.clone().into();
  let fir_str = fir_str.replace("{ctype T}", &ir_ty.ir_dump());
  let fir_str = fir_str.replace("{ftype T}", &cmtir_type_to_firrtl_type(ir_ty).to_string());

  external!(
    ["write_data".to_string(), "reset_value".to_string()],
    ["read_data".to_string()],
    Some("clock".to_string()),
    Some("reset".to_string()),
    fir_str.to_string(),
  );

  let read = ext_method!(
    None; None; false;
    () -> (io.out) {}
  );

  let write = ext_method!(
    Some("write_en".to_string()); None; true;
    (io.in_) {}
  );

  let reset = ext_method!(
    None; None;
    (io.init) {}
  );

  schedule!(reset, read, write);
}

// itfc_declare!(
//   param T;
//   /// A shift register with default value
//   pub struct ShiftReg {
//     /// input of shift reg
//     #[name("in")]
//     in_: input param T,
//     /// output of shift reg
//     out: output param T,
//   }
//   /// Write a value into the shift register
//   method write(in_);
//   /// Read a value from the shift register
//   method read() -> (out);
// );

/// Make a shift register, the `default` value will be pushed into the registers if `write` is not invoked.
#[module]
pub fn shift_reg(t: &Type, len: usize, default: u32) -> Reg {
  let io = io! {T: t.clone()};
  anno!("synthesis":"true");
  distinguisher!(&format!("shift_{len}_{default}"));
  let regs = (0..len).map(|i| named_instance!(format!("r_{}", i); reg(t))).collect::<Vec<_>>();
  let read = method! {
    () -> (io.out) {
      regs[len-1].read()
    }
  };
  let shift = always! {
    () {
      for i in (0..len-1).rev() {
        regs[i+1].write(regs[i].read());
      }
    }
  };
  let write = method! {
    (io.in_) {
      regs[0].write(io.in_);
    }
  };
  let write_default = always! {
    () {
      let default = default.lit(&t);
      regs[0].write(default);
    }
  };

  schedule!(read, shift, write, write_default);
}

/// Make a pipeline register, which delay the input for a cycle.
#[gen_fn]
pub fn reg_pipe(t: &Type, input: Var) -> Var {
  let r = named_instance!(format!("reg_pipe"); reg(t));
  r.write(input);
  r.read()
}

/// Make a named pipeline register, which delay the input for a cycle.
#[gen_fn]
pub fn named_reg_pipe(name: String, t: &Type, input: Var) -> Var {
  let r = named_instance!(name; reg(t));
  r.write(input);
  r.read()
}
