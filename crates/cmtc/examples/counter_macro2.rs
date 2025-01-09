use cmtc::ehdl::*;
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

  // let reg_i = Itfc::instantiate(&mut __cmt_gen, "reg_i".to_string(),
  // stl::reg(ty));
  let reg_i = instance!(stl::reg(ty));

  // rule inc always when!(i<lim) {
  //   i = i + 1
  // }
  let inc = always! {
    [reg_i.read().lt(literal(lim as i32, ty))]
    () {
      reg_i.write(reg_i.read() + literal(1, ty));
    }
  };

  // rule rst always when!(i>=lim) {
  //   i = 0
  // }
  let rst = always! {
    [reg_i.read().ge(literal(lim as i32, ty))]
    () {
      reg_i.write(literal(0, ty));
    }
  };

  // rule set(set_val) {
  //    i = set_val
  // }
  let set = method!(
    (io.set_val) {
      reg_i.write(io.set_val);
    }
  );

  // rule read() -> (count) {
  //    i.read()
  // }
  let read = method! {
    () -> (io.count) {
      reg_i.read().ast();
    }
  };

  method_rel!(set C set);

  schedule!(read, set, inc, rst);
}

fn main() -> anyhow::Result<()> {
  let ty = Type::UInt(4);
  let counter = make_counter(10, &ty);
  elaborate(counter, sv_config("counter.sv"))?;
  Ok(())
}
