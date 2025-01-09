use cmtc::ehdl::*;
use cmtc::*;
use cmtrs::*;
use utils::setup_logger;

itfc_declare! {
  struct SeqExample {
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method start();
}

#[module]
fn make_seq_example() -> SeqExample {
  let io = io! {};

  let t = Type::UInt(4);
  let r = instance!(stl::reg(&t));

  let out = method!(
    () -> (io.out) {
      ret!(r.read())
    }
  );

  let start = method!(
    fsm;
    () {
      seq!{
        step!{ r.write(literal(1, &t)); };
        step!{ r.write(literal(2, &t)); };
        seq!{
          step!{ r.write(literal(3, &t)); };
          seq!{
            step!{ r.write(literal(4, &t)); };
          }
          step!{ r.write(literal(5, &t)); };
        };
        step!{ r.write(literal(6, &t)); };
      }
    }
  );

  let r_default = always!(
    () {
      r.write(literal(0, &t));
    }
  );

  schedule!(out, start, r_default);
}

fn main() -> anyhow::Result<()> {
  setup_logger();
  let seq_example = make_seq_example();
  elaborate(seq_example, sv_config("tb/seq_example/seq_example.sv"))?;
  Ok(())
}
