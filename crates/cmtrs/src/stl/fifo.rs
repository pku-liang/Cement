//! Standard library for FIFOs
use super::*;
use crate as cmtrs;

itfc_declare! {
  /// Data type of the FIFO
  param T;
  /// A First-In-First-Out Queue, use [`FIFO::new()`] or various generator functions in [`stl::fifo`] to generate parametrized FIFOs.
  pub struct FIFO {
    #[name("in")]
    /// Input of the FIFO
    in_: input param T,
    /// Output of the FIFO
    out: output param T,
    /// Output whether the FIFO if full
    full: output Type::UInt(1)
  }
  /// Check if the FIFO is full
  method full()->(full);
  /// Put data of type T into FIFO, this method will not be fired if the FIFO is full
  method enq(in_);
  /// Get data of type T from FIFO, this method will not be fired if the FIFO is empty
  method deq()->(out);
}

impl FIFO {
  /// Make a default FIFO with depth n. Default FIFO behavior: independent
  pub fn new(depth: usize, t: &Type) -> Self { fifo_default(depth, t) }
}

/// FIFO with depth 1, enq depends on deq, actively push
#[module]
pub fn fifo1_push(t: &Type) -> FIFO {
  let io = io! { T: t };
  anno!("synthesis": "true");
  distinguisher!("1_push");

  let reg = instance!(Reg::new(t));
  let full_reg = instance!(Reg::new(&Type::UInt(1)));

  let deqed = instance!(Wire::new(&Type::UInt(1)));
  let enqed = instance!(Wire::new(&Type::UInt(1)));

  let full = method! {() -> (io.full){
    full_reg.read()
  }};

  let deq = method! {
    [full_reg.read()]
    () -> (io.out) {
      deqed.write(true);
      reg.read()
    }
  };

  let enq = method! {
    [!full_reg.read() | deqed.read()]
    (io.in_) {
      enqed.write(true);
      reg.write(io.in_);
    }
  };

  let deqed_default = always! {
    () {
      deqed.write(false);
    }
  };

  let enqed_default = always! {
    () {
      enqed.write(false);
    }
  };

  let next = always! {
    () {
      full_reg.write(enqed.read() | full_reg.read() & !deqed.read());
    }
  };

  schedule!(full, deq, enq, deqed_default, enqed_default, next);
}

#[test]
fn fifo1_pull_test() {
  use utils::setup_logger;
  setup_logger();
  let fifo = fifo1_pull(&Type::UInt(8));
  println!("{}", fifo.to_cmtir().ir_dump());
}

/// FIFO with depth 1, deq depends on enq, actively pull
#[module]
pub fn fifo1_pull(t: &Type) -> FIFO {
  let io = io! { T: t };
  anno!("synthesis": "true");
  distinguisher!("1_pull");

  let reg = instance!(Reg::new(t));
  let full_reg = instance!(Reg::new(&Type::UInt(1)));

  let deqed = instance!(Wire::new(&Type::UInt(1)));
  let enqed = instance!(Wire::new(&Type::UInt(1)));

  let full = method! {() -> (io.full){
    full_reg.read()
  }};

  let enq = method! {
    (io.in_) {
      enqed.write(true);
      reg.write(io.in_);
    }
  };

  let deq = method! {
    [full_reg.read() & enqed.read()]
    () -> (io.out) {
      deqed.write(true);
      reg.read()
    }
  };

  let enqed_default = always! {
    () { enqed.write(false); }
  };

  let deqed_default = always! {
    () { deqed.write(false); }
  };

  let next = always! {
    () {
      full_reg.write(enqed.read() | full_reg.read() & !deqed.read());
    }
  };

  schedule!(full, enq, deq, enqed_default, deqed_default, next);
}

/// FIFO with depth 2, fully independent, double buffered, throughput = 1 if not full
#[module]
pub fn fifo2_i(t: &Type) -> FIFO {
  let io = io! { T: t };
  anno!("synthesis": "true");
  distinguisher!("2_i");

  let reg0 = instance!(Reg::new(t));
  let reg1 = instance!(Reg::new(t));
  let state = instance!(Reg::new(&Type::UInt(2)));
  let deqed = instance!(Wire::new(&Type::UInt(1)));
  let enqed = instance!(Wire::new(&Type::UInt(1)));
  let enq_value = instance!(Wire::new(t));

  let full = method! {() -> (io.full){
    ret!(state.read().eq(literal(2, &Type::UInt(2))))
  }};

  let deq = method!([state.read().ne(literal(0, &Type::UInt(2)))]
    () -> (io.out) {
      deqed.write(true);
      reg0.read()
    }
  );

  let enq = method!([state.read().ne(literal(2, &Type::UInt(2)))]
    (io.in_) {
      enqed.write(true);
      enq_value.write(io.in_)
    }
  );

  let deqed_default = always! {
    () {
      deqed.write(false);
    }
  };

  let enqed_default = always! {
    () {
      enqed.write(false);
    }
  };

  let state0_update = always!(
    [state.read().eq(literal(0, &Type::UInt(2)))]
    () {
      if_!(enqed.read(){
        reg0.write(enq_value.read());
        state.write(literal(1, &Type::UInt(2)));
      });
    }
  );

  let state1_update = always!(
    [state.read().eq(literal(1, &Type::UInt(2)))]
    () {
      if_!(enqed.read(){
        if_!{deqed.read() {
          reg0.write(enq_value.read());
        } else {
          reg1.write(enq_value.read());
          state.write(literal(2, &Type::UInt(2)));
        }}
      } else {
        if_!{deqed.read() {
          state.write(literal(0, &Type::UInt(2)));
        } }
      });
    }
  );

  let state2_update = always! {
    [state.read().eq(literal(2, &Type::UInt(2)))]
    () {
      if_!{deqed.read() {
        reg0.write(reg1.read());
        state.write(literal(1, &Type::UInt(2)));
      }}
    }
  };

  method_rel!(state0_update CF state1_update);
  method_rel!(state0_update CF state2_update);
  method_rel!(state1_update CF state2_update);
  schedule!(full, deq, enq, deqed_default, enqed_default, state0_update, state1_update, state2_update);
}

/// Type of FIFO, used in [`fifo_unit`], [`fifo()`] and [`FIFO::new()`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FifoTy {
  Push,
  Pull,
  I,
}

/// Make one basic unit of FIFO
pub fn fifo_unit(fifo_ty: FifoTy, t: &Type) -> FIFO {
  match fifo_ty {
    FifoTy::Push => fifo1_push(t),
    FifoTy::Pull => fifo1_pull(t),
    FifoTy::I => fifo2_i(t),
  }
}

impl Default for FifoTy {
  /// Default is FifoTy::I
  fn default() -> Self { Self::I }
}

/// FIFO with depth and type
#[module]
pub fn fifo(depth: usize, fifo_ty: FifoTy, t: &Type) -> FIFO {
  let io = io! {T: t};
  anno!("synthesis": "true");

  match fifo_ty {
    FifoTy::Push => distinguisher!(&format!("{depth}_push")),
    FifoTy::Pull => distinguisher!(&format!("{depth}_pull")),
    FifoTy::I => distinguisher!(&format!("{depth}_i")),
  }

  assert!(depth > 0);
  let n = match fifo_ty {
    FifoTy::Push => depth,
    FifoTy::Pull => depth,
    FifoTy::I => depth.div_ceil(2),
  };
  let mut fifos = Vec::new();
  for i in 0..n {
    fifos.push(named_instance!(format!("fifo{i}");
      fifo_unit(fifo_ty, t)
    ));
  }

  let full = method! {() -> (io.full){
    ret!(fifos.iter().map(|f|f.full()).reduce(|a, b|a & b).unwrap())
  }};

  let deq = method! {
    () -> (io.out) {
      fifos.last().unwrap().deq()
    }
  };

  let mut forwards = Vec::new();
  for i in (1..n).rev() {
    forwards.push(named_always!(format!("forward{i}");
      () {
        fifos[i].enq(fifos[i-1].deq())
      }
    ));
  }

  let enq = method! {
    (io.in_) {
      fifos[0].enq(io.in_);
    }
  };

  let schedule: Vec<_> = chain!([full, deq], forwards, [enq]).collect();
  schedule_raw!(&schedule);
}

// TODO: mem based round fifo
// #[module]
// fn fifo_pass(depth: usize, t: Type) -> FIFO {
//   let io = io! {T: t.clone()};

//   let mut aw = depth.ilog2();
//   if (1 << aw) < depth {
//     aw += 1;
//   }
//   let at = Type::Int(aw);

//   let mem = mem1r1w(t.clone(), at, depth);
// }

/// Make a default FIFO with depth n. Default FIFO behavior: independent
pub fn fifo_default(depth: usize, t: &Type) -> FIFO { fifo(depth, FifoTy::default(), t) }
