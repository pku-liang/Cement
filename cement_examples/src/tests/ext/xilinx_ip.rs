use cmt::preclude::*;

#[interface(Default)]
struct Binary {
  a: B<8>,
  b: B<8>,
  c: <B<8> as Interface>::FlipT,
}

module! {
  Binary =>
  top(io) {
    io.c %= c.xilinx_add_ip(B8, io.a, io.b);
  }
}

#[test]
fn test_fixed_add() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Binary::default().top(&mut c);
  c.generate_workspace()
}