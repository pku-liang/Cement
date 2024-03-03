use cmt::preclude::*;
use cmt::simulator::*;

#[interface(Default)]
pub struct Pass {
  pub i: B<8>,
  pub o: Flip<B<8>>,
}

module! { Pass =>
    pass_m(module) {
        module.o %= module.i;
    }
}

#[interface(Default)]
pub(crate) struct TopPass {
  pass: Pass,
  i: B<8>,
  o: Flip<B<8>>,
}

module! { TopPass =>
    top_m(module) {
        let pass = instance!(pass_m(Pass::default()));
        let pass1 = instance!(pass_m(Pass::default()));
        module.pass %= pass;
        pass1.i %= module.i;
        module.o %= pass1.o + 1.lit(B8);
    }
}

#[test]
pub fn test_top_pass() -> Result<(), ()> {
  let mut c = Cmtc::new(CmtcConfig::default());

  TopPass::default().top_m(&mut c);

  c.print();
  

  Simulator::new(&c).test(async move |dut| {
    dut.poke("i", StateData::new_usize(0, 8));
    dut.poke("pass.i", StateData::new_usize(2, 8));
    dut.step().await;
    // println!("{:?} {:?}", dut.peek("o"), dut.peek("pass.o"));
    assert_eq!(dut.peek("o"), StateData::new_usize(1, 8));
    assert_eq!(dut.peek("pass.o"), StateData::new_usize(2, 8));
  });

  Ok(())
}

#[interface]
struct TopBits {
  i: Bits,
  o: Flip<Bits>,
}

impl TopBits {
  pub fn new(x: usize) -> Self { Self { i: Bits(x), o: Flip(Bits(x)) } }
}

module! {
    TopBits =>
    top_bits_m(module, x: i32) {
        if x%2 == 1 {
            module.o %= module.i;
        } else {
            module.o %= module.i.to_owned() + 1.lit(module.i.data_type());
        }
    }
}

#[test]
pub fn test_top_bits() -> Result<(), ()> {
  let mut c = Cmtc::new(CmtcConfig::default());

  TopBits::new(8).top_bits_m(&mut c, 4);

  // c.print();

  Simulator::new(&c).test(async move |dut| {
    dut.poke("i", StateData::new_usize(3, 8));
    dut.step().await;
    assert_eq!(dut.peek("o"), StateData::new_usize(4, 8));
  });

  // FIXME: reuse old c cause bug!!
  let mut c2 = Cmtc::new(CmtcConfig::default());
  TopBits::new(8).top_bits_m(&mut c2, 3);

  // c.print();

  Simulator::new(&c2).test(async move |dut| {
    dut.poke("i", StateData::new_usize(3, 8));
    dut.step().await;
    assert_eq!(dut.peek("o"), StateData::new_usize(3, 8));
  });
  Ok(())
}

#[interface]
struct ConcatTuple {
  i: (B<2>, B<3>, B<4>),
  o: Flip<Bits>,
}

impl ConcatTuple {
  fn new() -> Self { Self { i: (B2, B3, B4), o: Flip(Bits(9)) } }
}

module! {
    ConcatTuple =>
    concat_tuple_m(module) {
        module.o %= module.i.concat();
    }
}

#[test]
fn test_tuple_concat() {
  let mut c = Cmtc::new(CmtcConfig::default());

  ConcatTuple::new().concat_tuple_m(&mut c);

  c.print();

  Simulator::new(&c).test(async move |dut| {
    dut.poke("i.0", StateData::new_usize(0b11, 2));
    dut.poke("i.1", StateData::new_usize(0b001, 3));
    dut.poke("i.2", StateData::new_usize(0b1010, 4));
    dut.step().await;
    assert_eq!(dut.peek("o"), StateData::new_usize(0b110011010, 9));
  });
}

#[interface(Default, Copy)]
struct Extract2Bits {
  i: B<8>,
  o: Flip<B<2>>,
}

module! {
    Extract2Bits =>
    extract_2bits_m(module) {
        module.o %= module.i.extract(0, B2);
    }
}

#[test]
fn test_extract_2bits() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Extract2Bits::default().extract_2bits_m(&mut c);
  c.print();

  Simulator::new(&c).test(async move|dut|{
    dut.poke("i", StateData::new_usize(0b10110001, 8));
    dut.step().await;
    assert_eq!(dut.peek("o"), StateData::new_usize(0b01, 2));
  });
}

module! {
  Pass =>
  pass_with_wire_m(io) {
    let pass = mut_wire!(B::<8>);
    io.o %= pass.o;
    pass.i %= io.i;
  }
}

#[test]
fn test_pass_with_wire() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Pass::default().pass_with_wire_m(&mut c);
  c.print();

  Simulator::new(&c).test(async move|dut|{
    dut.poke("i", StateData::new_usize(98, 8));
    dut.step().await;
    assert_eq!(dut.peek("o"), StateData::new_usize(98, 8));
  });
}
