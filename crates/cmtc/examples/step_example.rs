use cmtc::*;
use cmtrs::*;
use utils::setup_logger;

itfc_declare! {
  struct StepExample {
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method start();
}

#[module]
fn make_step_example() -> StepExample {
  let io = io! {};

  let r = instance!(stl::reg(&Type::UInt(4)));

  let out = method!(
    () -> (io.out) {
      ret!(r.read())
    }
  );

  let start = method!(
    fsm;
    () {
      step!{ r.write(literal(1, &Type::UInt(4))); };
    }
  );

  let r_default = always!(
    () {
      r.write(literal(0, &Type::UInt(4)));
    }
  );

  schedule!(out, start, r_default);
}

fn main() -> anyhow::Result<()> {
  setup_logger();
  let step_example = make_step_example();
  // let mut circuit = step_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut circuit)?;
  // println!("{}", circuit.ir_dump());
  elaborate(step_example, sv_config("tb/step_example/step_example.sv"))?;
  Ok(())
}
