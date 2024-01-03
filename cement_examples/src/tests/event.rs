use __core::ops::Not;
use cmt::preclude::*;

#[interface(Default)]
struct ClkPass {
  i: B<8>,
  o: flip!(B<8>),
  clk: Clk,
}

module! {
    ClkPass(c) =>
    pass_odd_m(module) {
        let is_odd = module.i.extract(0, B1).eq(1.lit(B1));
        let reg = reg!(B8, module.clk);

        event! {
            ("store") =>
            reg.wr %= module.i;
            is_odd.to_owned()
        };

        module.o %= is_odd.mux(module.i, reg.rd);
    }
}

#[test]
fn test_clk_pass() {
  let mut c = Cmtc::new(CmtcConfig::default());
  ClkPass::default().pass_odd_m(&mut c);
  c.print();
}

#[test]
fn test_pass_odd_remove_event() {
  let mut c = Cmtc::new(CmtcConfig::default());
  ClkPass::default().pass_odd_m(&mut c);
  c.print_with_passes(vec![ReorderPass.into(), RemoveEventPass.into()], vec![
    c.module_op_id_iter().collect(),
    c.module_op_id_iter().collect(),
  ]);
}

#[test]
fn test_pass_odd_remove_event_select() {
  let mut c = Cmtc::new(CmtcConfig::default());
  ClkPass::default().pass_odd_m(&mut c);
  c.print_with_passes(
    vec![ReorderPass.into(), RemoveEventPass.into(), RemoveSelectPass.into()],
    vec![
      c.module_op_id_iter().collect(),
      c.module_op_id_iter().collect(),
      c.module_op_id_iter().collect(),
    ],
  );
}

module! {
  ClkPass =>
  pass_not_odd_m(io) {
    let not_odd = io.i.extract(0, B1).eq(1.lit(B1)).not();
    let reg = reg!(B8, io.clk);

    let store = event! {
      reg.wr %= io.i;
      not_odd
    };

    io.o %= store.mux(io.i, reg.rd);
  }
}

#[test]
fn test_pass_not_odd() {
  let mut cmtc = Cmtc::new(config!{
    workspace_dir => "./build"
  });
  ClkPass::default().pass_not_odd_m(&mut cmtc);
  cmtc.generate_workspace()
}
