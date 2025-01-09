use cmtc::*;
use cmtrs::stl::*;
use cmtrs::*;

itfc_declare!(
  param T;
  pub struct ErrTry{
    #[name("in")]
    in_: input param T,
    out: output param T,
  }
  method calc(in_) -> (out);
);

#[module]
fn err_type() -> ErrTry {
  let u4 = Type::UInt(4);
  let io = io! {T: &u4};

  let calc = method! {
    (io.in_) -> (io.out) {
      //   // thread 'main' panicked at crates/cmtrs/src/gen_ir.rs:203:21:
      //   // assertion failed: _ret.len() == expect_ret || _ret.len() == 0
      //   // FIXME: this is a bug?
      // if_! {
      //   &io.in_ {
      //     ret!(io.in_)
      //   } else {
      //     ret!(4.lit(&u4))
      //   }
      // }

      // Error: "IfOp's cond must be a bool [UInt(1)], but got i4."
      // let x = if_! {
      //   &io.in_ {
      //     ret!(io.in_)
      //   } else {
      //     ret!(4.lit(&u4))
      //   }
      // };
      // ret!(x)

      // Error: "type infer failed for IfOp, 0-th output of then-body and else-body have different/unmergeable types: i4 vs s3"
      // let x = if_! {
      //   (&io.in_ & 1.lit(&Type::UInt(1))).eq(&1.lit(&Type::UInt(1))) {
      //     ret!(io.in_)
      //   } else {
      //     ret!(3.lit(&Type::SInt(3)))
      //   }
      // };
      // ret!(x)

      // ret!(100.lit(&Type::UInt(8)))
      ret!(1.lit(&Type::SInt(8)))
    }
  };
  schedule!(calc);
}

#[module]
fn err_invoke() -> ErrTry {
  let u8 = Type::UInt(8);
  let io = io! {T: &u8};
  let mut r = instance!(reg(&u8));
  let calc = method! {
    (io.in_) -> (io.out) {
      // cannot be compiled, good
      // r %= r.read(io.in_);

      // Error: "type mismatch, expected i8 but got s3, for op (invoke () r.write (%3:s3)) {}'s 0-th output."
      r %= 1.lit(&Type::SInt(3));

      // r %= io.in_;
      ret!(r)

    }
  };
  schedule!(calc);
}

#[module]
fn err_guard() -> ErrTry {
  let u8 = Type::UInt(8);
  let io = io! {T: &u8};
  let mut r = instance!(reg(&u8));
  let calc = method! {
    [{r.write(1.lit(&u8)); r.read()}]
    (io.in_) -> (io.out) {
      r %= io.in_;
      ret!(r)
    }
  };
  schedule!(calc);
}

fn main() -> anyhow::Result<()> {
  // cmtir::setup_logger();
  // let et = err_type();
  // eprintln!("{}", et.to_cmtir().ir_dump());
  elaborate(err_type(), sv_config("err_type.sv"))?;
  elaborate(err_invoke(), sv_config("err_invoke.sv"))?;
  // let eg = err_guard();
  // eprintln!("{}", eg.to_cmtir().ir_dump());
  elaborate(err_guard(), sv_config("err_guard.sv"))?;
  Ok(())
}
