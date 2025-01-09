use cmtc::ehdl::*;
use cmtrs::*;

itfc_declare! {
  struct ForExample3 {
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method start ();
}

#[module]
fn make_for_example3() -> ForExample3 {
  let io = io! {};

  let t = Type::UInt(4);
  let mut i = instance!(stl::reg(&t));
  let j = instance!(stl::reg(&t));
  let out_wire = instance!(stl::wire(&t));

  let start = method!(
    fsm;
    () {
      for_!((
          i.write(literal(0, &t));
          true;
          i %= &i + literal(1, &t);
          i.read().lt(literal(3, &t))
        ) {
          for_!((
            j.write(literal(0, &t));
            true;
            j.write(j.read() + literal(1, &t));
            j.read().lt(literal(3, &t))
          ) {
            step!{
              out_wire.write(i.read() + j.read());
            };
          })
        })
    }
  );

  let out = method!(
    () -> (io.out) { ret!(out_wire.read()) }
  );

  schedule!(start, out);
}

fn main() -> Result<(), anyhow::Error> {
  let for_example3 = make_for_example3();
  utils::setup_logger();
  // use cmtc::*;
  // let mut ir = for_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut ir)?;
  // println!("{}", ir.ir_dump());

  elaborate(for_example3, sv_config("tb/for_example3/for_example3.sv"))?;
  Ok(())
}
