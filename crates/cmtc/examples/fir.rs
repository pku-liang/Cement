use cmtc::*;
use cmtrs::stl::*;
use cmtrs::*;

use rand::prelude::*;

itfc_declare!(
  param T;
  pub struct FIR {
    #[name("in")]
    in_: input param T,
    out: output param T,
  };
  method input(in_);
  method output () ->(out);
);

#[module]
fn fir_3(t: &Type, a0: i32, a1: i32, a2: i32) -> FIR {
  let io = io! {T: t};

  let mut reg0 = instance!(reg(t));
  let mut reg1 = instance!(reg(t));
  let mut reg2 = instance!(reg(t));

  let input = method!(
    (io.in_) {
      reg0 %= io.in_;
    }
  );

  let shift = always!(
    () {
      reg2 %= &reg1;
      reg1 %= &reg0;
    }
  );

  let output = method!(
    () -> (io.out) {
      ret!(&reg0 * a0.lit(t) + &reg1 * a1.lit(t) + &reg2 * a2.lit(t))
    }
  );

  schedule!(output, shift, input);
}

#[module]
fn gen_fir(t: &Type, a: &[i32]) -> FIR {
  let io = io! {T: t};

  let n = a.len();
  let regs: Vec<Reg> = (0..n).map(|_| instance!(reg(t))).collect();

  let input = method!(
    (io.in_) {
      regs[0].write(io.in_);
    }
  );

  let shift = always!(
    () {
      for i in 1..n {
        regs[i].write(&regs[i-1]);
      }
    }
  );

  let output = method!(
    () -> (io.out) {
      ret!(
        regs.iter().zip(a)
        .map(|(r, a)| r * a.lit(t))
        .reduce(|x, y| x+y )
        .unwrap()
      )
    }
  );

  schedule!(output, shift, input);
}

#[gen_fn]
fn adder_tree(t: &Type, inputs: &[Var]) -> Var {
  if inputs.len() == 1 {
    generate!(reg_pipe(t, inputs[0].clone()))
  } else if inputs.len() == 2 {
    generate!(reg_pipe(t, &inputs[0] + &inputs[1]))
  } else {
    let mid = inputs.len() / 2;
    let a = generate!(adder_tree(t, &inputs[..mid]));
    let b = generate!(adder_tree(t, &inputs[mid..]));
    generate!(reg_pipe(t, a + b))
  }
}

#[module]
fn gen_fir_addertree(t: &Type, a: &[i32]) -> FIR {
  let io = io! {T: t};

  let n = a.len();
  let regs: Vec<Reg> = (0..n).map(|_| instance!(reg(t))).collect();

  let input = method!(
    (io.in_) {
      regs[0].write(io.in_);
    }
  );

  let shift = always!(
    () {
      for i in 1..n {
        regs[i].write(&regs[i-1]);
      }
    }
  );

  let output = method!(
    () -> (io.out) {
      let muls: Vec<Var> = regs.iter().zip(a).map(|(r, a)| r * a.lit(t)).collect();
      ret!(generate!(adder_tree(t, &muls)))
    }
  );

  schedule!(output, shift, input);
}

itfc_declare!(
  struct TB {}
);

#[module]
fn make_tb(t: &Type, dut: FIR) -> TB {
  io! {};
  anno!("is_tb": "true");

  let dut = instance!(dut);
  let mut cycle = instance!(Integer::new());

  let mut rng = rand::thread_rng();

  for i in 0..10 {
    let input = named_always!(
      format!("input{i}");
      [cycle.eq(int(i))]
      () {
        let val: i8 = rng.gen();
        dut.input(val.lit(t));
        sim_print!("input: ", int(val));
      }
    );
  }

  let zeros = always!(
    [cycle.lt(int(20))]
    () { dut.input(0i16); }
  );

  let prints = always!(
    [cycle.lt(int(20))]
    () { sim_print!("cycle: ", cycle, " output: ", dut.output()) }
  );

  let exit = always!(
    [cycle.ge(int(20))]
    () {
      sim_exit!();
    }
  );

  let tick = always!(
    () { cycle %= &cycle + int(1); }
  );
}

fn main() -> anyhow::Result<()> {
  // utils::setup_logger();

  let t = Type::SInt(16);

  // Generate SVs
  let fir3 = fir_3(&t, 1, -1, 3);
  elaborate(fir3, sv_config("fir3.sv"))?;

  let fir5 = gen_fir(&t, &[1, 2, -2, -1, 4]);
  elaborate(fir5, sv_config("fir5.sv"))?;

  let fir8 = gen_fir_addertree(&Type::SInt(16), &[1, 3, 3, 5, -4, -3, -2, -1]);
  elaborate(fir8, sv_config("fir8.sv"))?;

  // Testbenches
  let fir3 = fir_3(&t, 1, -1, 3);
  let tb1 = make_tb(&t, fir3);
  elaborate(tb1, ksim_config("./tb/fir_tb1_ksim"))?;

  let fir5 = gen_fir(&t, &[1, 2, -2, -1, 4]);
  let tb2 = make_tb(&t, fir5);
  elaborate(tb2, ksim_config("./tb/fir_tb2_ksim"))?;

  let fir8 = gen_fir_addertree(&t, &[1, 3, 3, 5, -4, -3, -2, -1]);
  let tb3 = make_tb(&t, fir8);
  elaborate(tb3, ksim_config("./tb/fir_tb3_ksim"))?;
  Ok(())
}
