use cmtc::ehdl::*;
use cmtrs::*;

itfc_declare! {
  struct BranchExample {
    in_: input Type::UInt(8),
    out: output Type::UInt(8)
  }
  method out () -> (out);
  method start (in_);
}

#[module]
fn make_branch_example() -> BranchExample {
  let io = io! {};

  let t = Type::UInt(8);
  let r = instance!(stl::reg(&t));

  let out = method!(
    () -> (io.out) { ret!(r.read()) }
  );

  let start = method!(
    fsm;
    (io.in_) {
      for_!{(r.write(io.in_);true; ; r.read().gt(literal(1, &t))) {
        branch!{r.read() & literal(1, &t) {
          branch!{r.read().ne(1.lit(&t)){
            seq!{
              step!{ r.write(r.read() * literal(3, &t)); };
              step!{ r.write(r.read() + literal(1, &t)); };
            }
          } }
        } else {
          step!{ r.write(r.read() >> 1); };
        }
        };
      } }
    }
  );

  schedule!(out, start);
}

fn main() -> anyhow::Result<()> {
  let branch_example = make_branch_example();
  utils::setup_logger();
  // use cmtc::*;
  // let mut ir = branch_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut ir)?;
  // println!("{}", ir.ir_dump());

  elaborate(
    branch_example,
    sv_config("tb/branch_example/branch_example.sv"),
  )?;
  Ok(())
}
