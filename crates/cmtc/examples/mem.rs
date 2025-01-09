use cmtc::*;
use cmtrs::stl::*;
use cmtrs::*;

itfc_declare! {
  struct MemTest {
    mem: itfc Mem1r1wxk
  }
}

#[module]
fn test_mem1r1wx8() -> MemTest {
  let dt = Type::UInt(32);
  let at = Type::UInt(5);
  
  let io = io! { mem: mem1r1wxk(&dt, &at, 12, 8) };

  let _do_write = always! {
    () {
      io.mem.write(0, 1.lit(&dt), 0.lit(&at));
      io.mem.write(1, 2.lit(&dt), 1.lit(&at));
      io.mem.write(2, 3.lit(&dt), 2.lit(&at));
      io.mem.write(3, 4.lit(&dt), 3.lit(&at));
      io.mem.write(4, 5.lit(&dt), 4.lit(&at));
      io.mem.write(5, 6.lit(&dt), 5.lit(&at));
      io.mem.write(6, 7.lit(&dt), 6.lit(&at));
      io.mem.write(7, 8.lit(&dt), 7.lit(&at));
    }
  };

}


pub fn main() -> anyhow::Result<()> {
  let dt = Type::UInt(32);
  let at = Type::UInt(4);
  let depth = 12;

  let mem1r1wx4 = mem1r1wxk(&dt, &at, depth, 4);
  elaborate(mem1r1wx4, sv_config("mem1r1wx4.sv"))?;

  let test_mem1r1wx8 = test_mem1r1wx8();
  elaborate(test_mem1r1wx8, sv_config("mem1r1wx8_wrapped.sv"))?;

  let mem1r1w2dx4 = mem1r1w2dxk(&dt, &at, 8, 2, 4);
  elaborate(mem1r1w2dx4, sv_config("mem1r1w2dx4.sv"))?;

  Ok(())
}
