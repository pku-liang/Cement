use cmtc::*;
use cmtrs::*;

itfc_declare! {
  struct ForExample {
    out: output Type::UInt(4)
  }
  method out () -> (out);
  method start ();
}

#[module]
fn make_for_example() -> ForExample {
  let io = io! {};

  let mut r = instance!(stl::reg(&Type::UInt(4)));
  let mut sum = instance!(stl::reg(&Type::UInt(4)));

  let out = method!(
    () -> (io.out) {
      ret!(sum)
    }
  );

  let start = method!(
    fsm;
    () {
      seq!(
        for_!(
          ( r %= literal(0, &Type::UInt(4)); //init
            true; //init_cond
            r %= &r + literal(1, &Type::UInt(4)); //update
            r.lt(literal(3, &Type::UInt(4))) //update_cond
          ) { step!{ sum %= &sum + &r; }; }
        );
        step!{ sum %= 0.uint(4);};
        for_!(
          loose;
          ( r%= 0.uint(4); //init
            r.lt(literal(4, &Type::UInt(4))); //cond
            r %= &r + literal(1, &Type::UInt(4)) //update
          ) { step!{ sum %= &sum + &r; }; }
        );
      )
    }
  );

  schedule!(out, start);
}

fn main() -> anyhow::Result<()> {
  let for_example = make_for_example();
  utils::setup_logger();
  // utils::setup_logger_with_level("debug".to_string());
  // use cmtc::*;
  // let mut ir = for_example.to_cmtir();
  // FSMGenPass::new().apply_pass(&mut ir)?;
  // println!("{}", ir.ir_dump());

  elaborate(for_example, sv_config("tb/for_example/for_example.sv"))?;
  Ok(())
}
