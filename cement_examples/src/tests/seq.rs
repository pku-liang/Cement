use cmt::preclude::*;

#[interface(Default, Copy)]
pub struct Clked1To1<T: DataTypeTrait> {
  pub i: T,
  pub o: Flip<T>,
  pub clk: Clk,
}

impl<T: DataTypeTrait> Clked1To1<T> {
  pub fn new(x: T) -> Self { Self { i: x, o: Flip(x), clk: Clk::default() } }
}

module! { <T: DataTypeTrait> Clked1To1<T>(c) =>
    delay_m(module) {
        let delay = named!(module.i.reg(module.clk));
        module.o %= delay;
    }
}

module! { <T: DataTypeTrait> Clked1To1<T>(c) =>
    delay_k_m(module, k: u32) {
        let mut pre = module.i;
        for _ in 0..k {
            let delay = wire!(pre.reg(module.clk.to_owned()));
            pre = delay;
        }
        module.o %= pre;
    }
}

module! {
    <T: DataTypeTrait> Clked1To1<T> =>
    delay_instance_m(module) {
        let reg = reg!(module.i.to_owned().ifc(), module.clk);
        event! {
            ("always")<c> =>
            reg.wr %= module.i;
            1.lit(B1)
        };
        module.o %= reg.rd;
    }
}

#[test]
fn test_delay() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1::new(B4).delay_m(&mut c);
  c.print();
}

#[test]
fn test_delay_k() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1::new(B4).delay_k_m(&mut c, 3);
  c.print();
}

#[test]
fn test_delay_instance() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1::new(B4).delay_instance_m(&mut c);
  c.print();
}
