use cmtc::ehdl::*;
use cmtrs::*;

itfc_declare! {
  struct PipelineExample {
    in_: input Type::UInt(4),
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method input (in_);
}

#[module]
fn make_pipeline_example() -> PipelineExample {
  let io = io! {};

  let t = Type::UInt(4);
  let r1 = instance!(stl::reg(&t));
  let r2 = instance!(stl::reg(&t));
  let r3 = instance!(stl::reg(&t));

  let out = method!(
    () -> (io.out) {
      ret!(r3.read())
    }
  );

  let input = method!(
    pipeline;
    (io.in_) {
      step!{
        r1.write(io.in_ + literal(1, &t));
      }
      step!{
        r2.write(r1.read() + literal(1, &t));
      }
      step!{
        r3.write(r2.read() + literal(1, &t));
      }
    }
  );

  schedule!(out, input);
}

fn main() -> anyhow::Result<()> {
  utils::setup_logger();
  let pipeline_example = make_pipeline_example();
  // use cmtc::*;
  // let mut ir = pipeline_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut ir)?;
  // println!("{}", ir.ir_dump());

  elaborate(
    pipeline_example,
    sv_config("tb/pipeline_example/pipeline_example.sv"),
  )?;
  Ok(())
}
