use cmt::preclude::*;

#[interface(Default)]
struct Binary {
  a: B<8>,
  b: B<8>,
  c: <B<8> as Interface>::FlipT,
}

module_ext! {
  Binary =>
  ext_m(__) [
    tcl = TclIP::new_xilinx_ip("c_addsub", "add_fixed", "12.0", [("Implementation", "DSP48")
  ].into())] {}
}

module! {
  Binary =>
  instantiate_extern_m(io) {
    let ext = instance!(ext_m(Binary::default()));
    ext %= io;
  }
}

#[test]
fn test_instantiate_extern() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Binary::default().instantiate_extern_m(&mut c);
  c.generate_workspace()
}

mod xilinx_ip;