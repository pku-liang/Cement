use cmtc::ehdl::*;
use cmtc::*;
use cmtrs::*;
use utils::setup_logger;

itfc_declare! {
  struct OpExample {
    a: input Type::UInt(4),
    out: output Type::UInt(4)
  }
  method calc(a) -> (out);
}

#[module]
fn make_op_example() -> OpExample {
  let io = io! {};

  let calc = method!(
    (io.a) -> (io.out) {
      let add = &io.a + literal(1, &Type::UInt(1));
      let mul = add * literal(2, &Type::UInt(2));
      let rem = mul % literal(3, &Type::UInt(2));
      let shl = rem << 2;
      let dshr = shl.dshr(literal(2, &Type::UInt(2)));
      let neg = dshr.neg();
      let cvt = neg.cvt();
      let uint = cvt.as_uint();
      let not = uint.not();
      let or = &not | literal(1, &Type::UInt(1));
      let cat = or.cat(&io.a);
      let orr = cat.orr();
      let pad = orr.pad(8);
      let bits = pad.bits(3, 0);
      ret!(bits)
    }
  );
}

fn main() -> anyhow::Result<()> {
  let op_example = make_op_example();
  elaborate(op_example, sv_config("op_example.sv"))?;
  Ok(())
}
