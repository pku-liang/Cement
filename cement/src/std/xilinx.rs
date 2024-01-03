mod arith;
mod ram;

pub use arith::*;
use cmt_macros::cmt_fn;
pub use ram::*;

use crate::preclude::*;

pub struct CfgXilinxIP {
  pub latency_read: u32,
  pub latency_fixed_add: u32, 
}

impl Into<CfgValue> for CfgXilinxIP {
  fn into(self) -> CfgValue {
      CfgValue::XilinxIp(self)
  }
}

impl Cmtc {
  #[cmt_fn(self)]
  pub fn xilinx_add_ip<T: DataTypeTrait>(&mut self, data_type: T, a: I<T>, b: I<T>) -> I<T> {
    let add_ip = instance!(add_fixed(FixedAdd::new(data_type), self.config.xilinx_ip_config.latency_fixed_add));
 
    add_ip.A %= a;
    add_ip.B %= b;
    add_ip.S
  }
}