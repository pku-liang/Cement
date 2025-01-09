use cmtc::ehdl::*;
use cmtrs::*;

itfc_declare! {
  struct ForExample2 {
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method start ();
}

#[module]
fn make_for_example2() -> ForExample2 {
  let io = io! {};

  let t = Type::UInt(4);
  let mut i = instance!(stl::reg(&t));
  let j = instance!(stl::reg(&t));
  let out_wire = instance!(stl::wire(&t));

  let start = method!(
    fsm;
    () {
      for_!(loose;(
          i.write(literal(0, &t));
          i.read().lt(literal(4, &t));
          i %= &i + literal(1, &t)
        ) {
          for_!(loose;(
            j.write(literal(0, &t));
            j.read().lt(literal(4, &t));
            j.write(j.read() + literal(1, &t))
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

fn main() -> anyhow::Result<()> {
  let for_example2 = make_for_example2();
  utils::setup_logger();
  // use cmtc::*;
  // let mut ir = for_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut ir)?;
  // println!("{}", ir.ir_dump());

  elaborate(for_example2, sv_config("tb/for_example2/for_example2.sv"))?;
  Ok(())
}
