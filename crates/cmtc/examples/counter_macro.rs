use cmtc::{elaborate, fir_config};
use cmtrs::*;

itfc_declare! {
  param T;
  struct Counter {
    set_val: input param T,
    count: output param T,
  }
  method read() -> (count);
  method set(set_val);
}

#[module]
fn make_counter(lim: usize, ty: &Type) -> Counter {
  let io = io! {
    T: ty
  };
  anno!("synthesis": "true");

  let reg_i = instance!(stl::reg(ty));

  let inc = always! {
    () {
      reg_i.write( if_!(
        reg_i.read().lt(literal(lim as i32, ty)) {
          let x = var!(reg_i.read() + literal(1, ty));
          ret!(x);
        } else {
          ret!(literal(0, ty));
        }
      ))
    }
  };

  let set = method!(
    (io.set_val) {
      reg_i.write(io.set_val);
    }
  );

  let read = method! {
    () -> (io.count) {
      reg_i.read();
    }
  };

  method_rel!(set C set);

  schedule!(inc, set, read);
}

fn main() -> anyhow::Result<()> {
  let ty = Type::UInt(4);
  let counter = make_counter(10, &ty);
  // println!("{}", counter.to_cmtir().ir_dump());
  elaborate(counter, fir_config("counter.fir")).unwrap();
  Ok(())
}
