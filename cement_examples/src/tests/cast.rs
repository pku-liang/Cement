use cmt::preclude::*;

use super::basics::Pass;

module! { Pass(c) =>
    cast_b_bits_m(module) {
        let bits = wire!(module.i.cast(Bits(8)) + 1.lit(Bits(8)));
        module.o %= bits.cast(B8);
    }
}

#[test]
fn test_cast_b_bits() {
  let mut c = Cmtc::new(CmtcConfig::default());
  Pass::default().cast_b_bits_m(&mut c);
  c.print();
}
