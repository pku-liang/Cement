use crate::preclude::*;

#[StructFields(pub)]
#[interface]
pub struct Reg<T: SignalTrait> {
  w_port: T,
  r_port: <T as Interface>::FlipT,
}

#[StructFields(pub)]
#[interface]
pub struct RegClk<T: SignalTrait> {
  clk: Clk,
  w_port: T,
  r_port: <T as Interface>::FlipT,
}

impl<T: SignalTrait> RegClk<T> {
  pub fn new(t: T) -> Self {
    Self {
      clk: Clk,
      w_port: t.to_owned(),
      r_port: t.flip(),
    }
  }
}
module! {
  <T: SignalTrait> RegClk<T> =>
  reg_clk_m(reg) {
      reg.r_port %= reg.w_port.expr().reg(reg.clk);
  }
}


impl<T: SignalTrait> RegClkFlipImpl<T> {
  pub fn set_clk<C: ToExpr<Clk>>(self, clk_value: C, c: &mut Cmtc) -> RegFlipImpl<T> {
    let RegClkFlipImpl { clk, w_port, r_port } = self;
    clk.connect_expr(clk_value.expr(), c);
    RegFlipImpl { w_port, r_port }
  }
}

#[StructFields(pub)]
pub struct RegAccess<T: SignalTrait> {
  wr: <<T as Interface>::FlipT as Interface>::ImplT,
  rd: <T as Interface>::ImplT,
}

impl <T: SignalTrait> RegFlipImpl<T> {
  #[cmt_fn(c)]
  #[track_caller]
  pub fn hold(self, c:&mut Cmtc) -> RegAccess<T> {
    let RegFlipImpl {w_port, r_port} = self;
      let wr = mut_wire!(w_port.ifc().flip());
      w_port %= select(false, vec![None], vec![wr.o.expr()], Some(r_port.expr()));
      RegAccess {
        wr: wr.i,
        rd: r_port,
      }
  }
}

