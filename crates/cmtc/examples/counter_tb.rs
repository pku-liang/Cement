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

itfc_declare!(
  struct Tb {}
);

#[module]
fn make_tb() -> Tb {
  io! {}
  anno!("is_tb": "true");

  let counter = instance!(make_counter(10, &Type::UInt(4)));
  let mut cycle = instance!(stl::integer());

  let sim = always!(
    [cycle.lt(stl::int(20))]
    () {
      cycle %= &cycle + stl::int(1);
      sim_print!("cycle: ", cycle, "count: ", counter.read());
    }
  );

  let exit = always!(
    [cycle.ge(stl::int(20))]
    () {
      sim_exit!();
    }
  );

  method_rel!(sim CF exit);
}

fn main() -> anyhow::Result<()> {
  let tb = make_tb();
  println!("{}", tb.to_cmtir().ir_dump());
  // let ToFirProduct { firrtl, .. } = counter.to_firrtl()?;
  // println!("{}", firrtl);
  Ok(())
}
