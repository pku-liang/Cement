use cmtc::*;
use cmtrs::*;

itfc_declare! {

  struct RegInitTest{
    y: output Type::UInt(4),
  }
  method read() -> (y);
}

#[module]
fn reg_init_test() -> RegInitTest {
  let io = io! {};
  let reg = instance!(stl::reg_init(&Type::UInt(4), 2));

  let read = method!(
    () -> (io.y) {
      reg.read()
    }
  );

  let update = always! {
    () {
      reg.write(reg.read() + literal(1, &Type::UInt(4)));
    }
  };

  schedule!(read, update);
}

fn main() -> anyhow::Result<()> {
  utils::setup_logger();
  let mut reg = reg_init_test();
  // let mut reg = stl::reg_init(Type::Int(4));
  reg.set_name("RegInit".to_string());
  elaborate(reg, sv_config("tb/reg/reg_init.sv"))?;
  Ok(())
}
