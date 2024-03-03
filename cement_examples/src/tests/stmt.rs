use cmt::preclude::*;
use cmt::simulator::StateData;

use super::seq::Clked1To1;

#[interface(Default)]
pub struct Clked1To1GoDone {
  content: Clked1To1<B<8>>,
  protocol: GoDoIfc,
}

module! {
    Clked1To1GoDone(c) =>
    sum_k_m(module, k: u32) {
        let sum = reg!(B8, module.content.clk.to_owned());

        // FIXME: if this assigns is later than the write event, sum.rd is moved!
        module.content.o %= sum.rd;
        let write = event! {
          sum.wr %= sum.rd + module.content.i;
        };


        let v_step: Vec<_> = (0..k).into_iter().map(|k| {
            Stmt {
                name: Some(format!("step{}", k)),
                ast: StmtAst::Step(StepStmt {
                    events: vec![write.to_owned()],
                    wait_at_exit: Vec::new(),
                })
            }
        }).collect();

        let seq = Stmt {
            name: Some(format!("seq")),
            ast: StmtAst::Seq(SeqStmt {
                stmts: v_step
            })
        };

        let go_event = event!(module.protocol.go);
        let done_event = event!();
        c.synthesize(seq, GoDone::new(
            module.content.clk, go_event, done_event.to_owned()
        ));
        module.protocol.done %= done_event;
    }

}

#[test]
fn print_sum_k_m() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1GoDone::default().sum_k_m(&mut c, 3);

  c.elaborate();
  c.print();
}

#[test]
fn test_sum_k_m() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1GoDone::default().sum_k_m(&mut c, 3);

  c.simulate(async move |dut| {
    dut.keep_poke("content.i", StateData::new_usize(3, 8));
    dut.poke("protocol.go", StateData::new_bool(true));
    dut.step().await;
    assert_eq!(dut.peek("protocol.done"), StateData::new_bool(false));
    dut.step().await;
    assert_eq!(dut.peek("protocol.done"), StateData::new_bool(false));
    dut.step().await;
    assert_eq!(dut.peek("protocol.done"), StateData::new_bool(false));
    dut.step().await;
    assert_eq!(dut.peek("protocol.done"), StateData::new_bool(true));
    assert_eq!(dut.peek("content.o"), StateData::new_usize(9, 8));
  });
}

module! {
    Clked1To1GoDone(c) =>
    for_if_sum_m(module, n: usize) {
        let clk = module.content.clk;
        let sum = reg!(B8, clk.to_owned());
        let i = reg!(Bits(clog2(n)), clk.to_owned());

        let sum0 = mut_wire!(sum.wr.ifc().flip());
        let sum1 = mut_wire!(sum.wr.ifc().flip());

        sum.wr %= select(false, vec![None, None], vec![sum0.o.expr(), sum1.o.expr()], None);

        let accumulate_shl1 = event! { ("acc_shl1") =>
            sum0.i %= sum.rd.to_owned() + (module.content.i.to_owned() >> 1.lit(B8));
        };

        let accumulate = event! {
            ("acc") =>
            sum1.i %= sum.rd + module.content.i.to_owned();
        };

        let if_cond = event!(module.content.i.extract(0, B1).eq(1.lit(B1)));
        let then_step = Stmt {
            name: Some("then_step".to_string()),
            ast: StmtAst::Step(StepStmt { events: vec![accumulate], wait_at_exit: Vec::new() })
        };
        let else_step = Stmt {
            name: Some("else_step".to_string()),
            ast: StmtAst::Step(StepStmt { events: vec![accumulate_shl1], wait_at_exit: Vec::new() })
        };

        let for_stmt = Stmt {
            name: Some("for".to_string()),
            ast: StmtAst::For(ForStmt {
                indvar_rd: i.rd.v_ir_entity_id()[0].unwrap(),
                indvar_wr: i.wr.v_ir_entity_id()[0].unwrap(),
                start: Bound::Const(0),
                end: Bound::Const(n),
                incr: true,
                step: 1,
                do_stmt: Box::new(
                    Stmt {
                        name: Some("if".to_string()),
                        ast: StmtAst::If(IfStmt { cond: if_cond, then_stmt: Box::new(then_step), else_stmt: Some(Box::new(else_step)) })
                    }
                )
            })
        };

        let go_event = event!(module.protocol.go);
        let done_event = event!();
        module.protocol.done %= done_event.to_owned();
        c.synthesize(for_stmt, GoDone::new(
            clk, go_event, done_event)
        );
    }
}

#[test]
fn test_for_if_sum_m() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1GoDone::default().for_if_sum_m(&mut c, 4);
  c.print();
}

module! {
  Clked1To1GoDone(c) =>
  for_if_sum_macro_m(module, n: usize) {
    let clk = module.content.clk;
    let sum = reg!(B8, clk.to_owned());
    let i = reg!(Bits(clog2(n)), clk.to_owned());

    let sum0 = mut_wire!(sum.wr.ifc().flip());
    let sum1 = mut_wire!(sum.wr.ifc().flip());

    sum.wr %= select(false, vec![None, None], vec![sum0.o.expr(), sum1.o.expr()], None);

    let accumulate_shl1 = event! { ("acc_shl1") =>
        sum0.i %= sum.rd.to_owned() + (module.content.i.to_owned() >> 1.lit(B8));
    };

    let accumulate = event! {
        ("acc") =>
        sum1.i %= sum.rd + module.content.i.to_owned();
    };

    let if_cond = event!(module.content.i.extract(0, B1).eq(1.lit(B1)));

    let stmt = stmt! {
      for i.rd.v_ir_entity_id()[0].unwrap(), i.wr.v_ir_entity_id()[0].unwrap(), Bound::Const(0), Bound::Const(n), true, 1 =>
        if if_cond =>
          accumulate
        else
          accumulate_shl1
    };

    let go_event = event!(module.protocol.go);
    let done_event = event!();
    module.protocol.done %= done_event.to_owned();
    c.synthesize(stmt, GoDone::new(
        clk, go_event, done_event)
    );

  }
}

#[test]
fn test_for_if_sum_macro_m() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Clked1To1GoDone::default().for_if_sum_macro_m(&mut c, 4);
  c.elaborate();
  c.print();
  // c.print_common();
}

#[interface(Default)]
struct ValidB8 {
  data: B<8>,
  valid: B<1>,
}

#[interface(Default)]
struct ClkedDyn1To1 {
  i: ValidB8,
  o: flip!(ValidB8),
  clk: Clk,
  protocol: GoDoIfc,
}

module! {
  ClkedDyn1To1(c) =>
  while_sum_dyn_m(module, n: usize) {
    let bits_n = Bits(clog2(n+1));
    let indvar = reg!(bits_n, module.clk.to_owned());
    let sum = reg!(B8, module.clk.to_owned());
    let cond = wire!(indvar.rd.to_owned().lt(n.lit(bits_n)));


    let accumulate = event! {
      ("acc") =>
      sum.wr %= sum.rd.to_owned() + module.i.data.to_owned();
      indvar.wr %= indvar.rd + 1.lit(bits_n);
    };

    let output = event! {
      ("output") =>
      module.o.data %= sum.rd.to_owned();
      module.o.valid %= true.lit(B1);
    };

    let cond_event = event!(cond);
    let i_valid_event = event!(module.i.valid);

    let stmt = stmt! {
      seq {
        {
          while cond_event =>
            accumulate; [i_valid_event]
        }
        { output }
      }
    };

    let go_event = event!(module.protocol.go);
    let done_event = event!();
    module.protocol.done %= done_event.to_owned();
    c.synthesize(stmt, GoDone::new(
        module.clk, go_event, done_event)
    );
  }
}

#[test]
fn test_while_sum_dyn_m() {
  let mut c = Cmtc::new(CmtcConfig::default());
  ClkedDyn1To1::default().while_sum_dyn_m(&mut c, 4);
  c.print();
  // c.print_common();
}
