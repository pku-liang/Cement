use cmtc::*;
use cmtrs::*;

itfc_declare! {
  param T;
  struct SeqMerge {
    a: itfc stl::FIFO{T: param T},
    b: itfc stl::FIFO{T: param T},
    c: itfc stl::FIFO{T: param T},
  }
}

#[module]
fn seq_merge(t: &Type) -> SeqMerge {
  let io = io! {
    T: t,
    a: stl::fifo(4, stl::FifoTy::I, t),
    b: stl::fifo(4, stl::FifoTy::I, t),
    c: stl::fifo(4, stl::FifoTy::I, t)
  };
  // let io = io! {T: t, a: stl::fifo1_d(t), b:
  // stl::fifo1_d(t), c: stl::fifo2_i(t)};
  anno!("synthesis":"true");

  let reg_a = instance!(stl::Reg::new(t));
  let reg_b = instance!(stl::Reg::new(t));

  let reg_a_valid = instance!(stl::Reg::new(&Type::UInt(1)));
  let reg_b_valid = instance!(stl::Reg::new(&Type::UInt(1)));

  let max_val = match t {
    Type::UInt(n) => (1 << n) - 1,
    _ => panic!("Not supported"),
  };

  let in_a = always!(
    [!reg_a_valid.read() | reg_a.read().eq(literal(max_val, t))]
    () {
      reg_a.write(io.a.deq());
      reg_a_valid.write(true);
    }
  );
  let in_b = always!(
    [!reg_b_valid.read()| reg_b.read().eq(literal(max_val, t))]
    () {
      reg_b.write(io.b.deq());
      reg_b_valid.write(true);
    }
  );

  let out = always! {
    [reg_a_valid.read() & reg_b_valid.read() & !io.c.full()]
    () {
      let a = var!(reg_a.read());
      let b = var!(reg_b.read());
      if_!{(&a).le(&b) {
        io.c.enq(a);
        reg_a_valid.write(false);
      } else {
        io.c.enq(b);
        reg_b_valid.write(false);
      } }
    }
  };

  schedule!(out, in_a, in_b);
}

itfc_declare!(
  param T;
  struct Top {
    in_a: input param T,
    in_b: input param T,
    out_c: output param T,
  };
  method input_a(in_a);
  method input_b(in_b);
  method output_c()->(out_c);
);

#[module]
fn make_top(t: &Type) -> Top {
  let io = io! {T: t};
  anno!("synthesis":"true");

  let merger = instance!(seq_merge(t));

  let input_a = method! {
    [!merger.a.full()]
    (io.in_a) {
      merger.a.enq(io.in_a);
    }
  };

  let input_b = method! {
    [!merger.b.full()]
    (io.in_b) {
      merger.b.enq(io.in_b);
    }
  };

  let output_c = method! {
    () -> (io.out_c) {
      merger.c.deq()
    }
  };

  schedule!(input_a, input_b, output_c);
}

fn main() -> anyhow::Result<()> {
  // utils::setup_logger_with_level("debug".to_string());
  utils::setup_logger();
  let mut top = make_top(&Type::UInt(8));
  top.set_name("Top".to_string());

  // let circuit = top.to_cmtir();
  // println!("{}", circuit.ir_dump());

  elaborate(top, sv_config("tb/seq_merge/seq_merge.sv"))?;
  Ok(())
}
