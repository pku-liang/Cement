use cmtir::anyhow;
use cmtrs::*;
use cmtsim::{ksim_config, elaborate};
use utils::{setup_logger, setup_logger_with_level};

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

  schedule!(set, inc, read);
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

  let init = always!(
    [cycle.eq(stl::int(0))]
    () {
      counter.set(stl::int(0));
      sim_print!("cycle: ", cycle, ", count: ", counter.read(), ", set 0");
      cycle %= &cycle + stl::int(1);
    }
  );
  let sim = always!(
    [cycle.lt(stl::int(20)) & cycle.gt(stl::int(0))]
    () {
      sim_print!("cycle: ", cycle, ", count: ", counter.read());
      cycle %= &cycle + stl::int(1);
    }
  );

  let exit = always!(
    [cycle.ge(stl::int(20))]
    () {
      sim_exit!();
    }
  );

  method_rel!(init C sim);
  schedule!(init, sim, exit);
}

fn main() -> anyhow::Result<()> {
  setup_logger_with_level("debug".to_string());
  let tb = make_tb();
  elaborate(tb, ksim_config("./tb/counter_ksim"))
}
