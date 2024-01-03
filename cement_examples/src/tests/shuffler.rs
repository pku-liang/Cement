use __core::ops::Not;
use cmt::preclude::*;

#[interface(Default)]
pub struct Pkt<T: DataTypeTrait, const N: usize>
where [(); clog2(N)]: Sized
{
  data: T,
  dest: B<{ clog2(N) }>,
  valid: B<1>,
}

impl<T: DataTypeTrait, const N: usize> std::clone::Clone for PktImpl<T, N>
where [(); clog2(N)]: Sized
{
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
      dest: self.dest.clone(),
      valid: self.valid.clone(),
    }
  }
}

#[interface]
pub struct Arbiter<const N: usize, T: DataTypeTrait>
where [(); clog2(N)]: Sized
{
  pkt: [Pkt<T, N>; N],
  resend: [PktFlip<T, N>; N],
  pkt_out: [PktFlip<T, N>; N],
  sel: [B<{ clog2(N) }>; N],
}

impl<const N: usize, T: DataTypeTrait + Default> Default for Arbiter<N, T>
where [(); clog2(N)]: Sized
{
  fn default() -> Self {
    Self {
      pkt: array![Default::default(); N],
      resend: array![Default::default(); N],
      pkt_out: array![Default::default(); N],
      sel: array![Default::default(); N],
    }
  }
}

module! {
    <const N: usize, T: DataTypeTrait> Arbiter<N, T> where [(); clog2(N)]: Sized =>
    arbiter_m(_module) {
    }
}

type BFlip<const N:usize> = <B<N> as Interface>::FlipT;
#[interface]
pub struct PktxN<const N: usize, T: DataTypeTrait>
where [(); clog2(N)]: Sized
{
  clk: Clk,
  i: [Pkt<T, N>; N],
  i_ready: [BFlip<1>; N],
  o: [PktFlip<T, N>; N],
}

impl<const N: usize, T: DataTypeTrait + Default> Default for PktxN<N, T>
where [(); clog2(N)]: Sized
{
  fn default() -> Self {
    Self {
      clk: Default::default(),
      i: array![Default::default(); N],
      i_ready: array![Default::default(); N],
      o: array![Default::default(); N],
    }
  }
}

module! {
    <const N:usize, T: DataTypeTrait+Default> PktxN<N, T> where [(); clog2(N)]: Sized =>
    shuffler_m(module) {
        let arbiter = instance!(arbiter_m(Arbiter::<N, T>::default()));
        // let resend_wire = instance!(wire(Wire::new(module.ins.ifc())));
        let resend_fwd = mut_wire!(arbiter.resend.ifc());

        let xbar = event! {};
        let receive = event! {
            resend_fwd.i %= arbiter.resend;
        };
        let wait = event! {};
        let send = event! {
            for (pkt, ready, resend, arb_pkt) in multizip((module.i, module.i_ready, resend_fwd.o, arbiter.pkt)) {
                ready %= receive.clone().mux(
                    resend.valid.clone().not(),
                    1.lit(B1)
                );
                arb_pkt %= receive.clone().mux(
                    resend.valid.clone().mux(
                        resend,
                        pkt.clone()
                    ),
                    pkt
                );
            };
        };

        let send_step = Stmt {
            name: Some(format!("send_step")),
            ast: StmtAst::Step(StepStmt {
                events: vec![send],
                wait_at_exit: Vec::new(),
            })
        };
        let wait_step = Stmt {
            name: Some(format!("wait_step")),
            ast: StmtAst::Step(StepStmt {
                events: vec![wait],
                wait_at_exit: Vec::new(),
            })
        };

        let receive_step = Stmt {
            name: Some(format!("receive_step")),
            ast: StmtAst::Step(StepStmt {
                events: vec![receive],
                wait_at_exit: Vec::new(),
            })
        };

        let xbar_step = Stmt {
            name: Some(format!("xbar_step")),
            ast: StmtAst::Step(StepStmt {
                events: vec![xbar],
                wait_at_exit: Vec::new(),
            })
        };

        let pipeline = Stmt {
            name: Some(format!("pipeline")),
            ast: StmtAst::Seq(SeqStmt {
                stmts: vec![send_step, wait_step, receive_step, xbar_step]
            })
        };

        c.synthesize(pipeline, Pipeline::new(module.clk, 1));
    }
}

#[test]
fn test_shuffler() {
  let mut c = Cmtc::new(CmtcConfig::default());
  PktxN::<4, B<8>>::default().shuffler_m(&mut c);
  c.print();
}
