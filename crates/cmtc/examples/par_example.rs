use cmtc::ehdl::*;
use cmtc::*;
use cmtrs::*;
use utils::setup_logger;

itfc_declare! {
  struct ParExample {
    out1: output Type::UInt(4),
    out2: output Type::UInt(4),
    out3: output Type::UInt(4),
    out4: output Type::UInt(4),
  }
  method out () -> (out1, out2, out3, out4);
  method start();
}

#[module]
fn make_par_example() -> ParExample {
  let io = io! {};

  let t = Type::UInt(4);
  let w1 = instance!(stl::reg(&t));
  let w2 = instance!(stl::reg(&t));
  let w3 = instance!(stl::reg(&t));
  let w4 = instance!(stl::reg(&t));

  let start = method!(
    fsm;
    () {
      seq!{
        par!{
          step!{ w1.write(literal(1, &t)); };
          seq!{
            par!{
              step!{w2.write(literal(1, &t));};
              seq!{
                step!{w3.write(literal(1, &t));};
                step!{w3.write(literal(2, &t));};
              }
            }
            seq!{
              step!{ w2.write(literal(2, &t)); };
              step!{ w2.write(literal(3, &t)); };
            };
          };
          seq!{
            step!{ w4.write(literal(1, &t)); };
            step!{ w4.write(literal(2, &t)); };
            step!{ w4.write(literal(3, &t)); };
            step!{ w4.write(literal(4, &t)); };
          }
        };
        step!{
          w1.write(literal(5, &t));
          w2.write(literal(5, &t));
          w3.write(literal(5, &t));
          w4.write(literal(5, &t));
        };
        par!{
          step!{w1.write(literal(6, &t));};
          step!{w2.write(literal(6, &t));};
          step!{w3.write(literal(6, &t));};
          step!{w4.write(literal(6, &t));};
        };
      };
    }
  );

  let w1_default = always!( () { w1.write(literal(0, &t)); });
  let w2_default = always!( () { w2.write(literal(0, &t)); });
  let w3_default = always!( () { w3.write(literal(0, &t)); });
  let w4_default = always!( () { w4.write(literal(0, &t)); });

  let out = method!(
    () -> (io.out1, io.out2, io.out3, io.out4) {
      ret!((w1, w2, w3, w4))
    }
  );

  schedule!(start, w1_default, w2_default, w3_default, w4_default, out);
}

fn main() -> anyhow::Result<()> {
  setup_logger();
  let par_example = make_par_example();

  elaborate(par_example, sv_config("tb/par_example/par_example.sv"))?;
  Ok(())
}
