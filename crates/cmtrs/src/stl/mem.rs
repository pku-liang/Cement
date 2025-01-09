//! Standard library for memories

use super::*;
use crate as cmtrs;

itfc_declare! {
  /// Date type of the memory
  param DT;
  /// Address type of the memory
  param AT;

  /// Memory, 1R1W, read & write both synchronous
  pub struct Mem1r1w {
    /// Write data
    wdata: input param DT,
    // Write Address
    waddr: input param AT,
    /// Read Addresss
    raddr: input param AT,
    // Read data
    rdata: output param DT,
  }
  /// Start reading data at raddr. Invoke [`rd1()`](`Mem1r1w::rd1()`) at the next cycle to get the result.
  method rd0(raddr) -> ();
  /// Get the result of last [`rd0()`](`Mem1r1w::rd0()`) invoke at the last cycle.
  method rd1() -> (rdata);
  /// Write data into memory
  method write(wdata, waddr) -> ();
}

/// Memory, read & write both synchronous, i.e. latency = 1
#[module]
pub fn mem1r1w(dt: &Type, at: &Type, depth: usize) -> Mem1r1w {
  let io = io! {DT: dt, AT: at};
  anno!("synthesis": "true");
  distinguisher!(&format!("{depth}"));
  let fir_str = r#"
module Mem1r1w_{ctype D}_{ctype A}_{const N}:
    input clk: Clock
    input en: UInt<1>
    input raddr: {type A}
    output rd1_valid: UInt<1>
    output rdata: {type D}
    input wen: UInt<1>
    input waddr: {type A}
    input wdata: {type D}

    reg r: UInt<1>, clk
    r <= en
    mem mymemory:
        data-type => {type D}
        depth => {const N}
        read-latency => 1
        write-latency => 1
        reader => r
        writer => w
        read-under-write => old

    mymemory.r.en <= en
    mymemory.r.clk <= clk
    mymemory.r.addr <= raddr
    rdata <= mymemory.r.data
    rd1_valid <= r

    mymemory.w.en <= wen
    mymemory.w.clk <= clk
    mymemory.w.addr <= waddr
    mymemory.w.data <= wdata
    mymemory.w.mask <= UInt<1>(1)
"#;
  let dt: ir::Type = dt.clone().into();
  let at: ir::Type = at.clone().into();
  let fir_str = fir_str.replace("{ctype D}", &dt.ir_dump());
  let fir_str = fir_str.replace("{ctype A}", &at.ir_dump());
  let fir_str = fir_str.replace("{const N}", &depth.to_string());
  let fir_str = fir_str.replace("{type D}", &cmtir_type_to_firrtl_type(dt).to_string());
  let fir_str = fir_str.replace("{type A}", &cmtir_type_to_firrtl_type(at).to_string());

  external!(
    ["wdata".to_string(), "waddr".to_string(), "raddr".to_string()],
    ["rdata".to_string()],
    Some("clk".to_string()),
    None,
    fir_str.to_string()
  );

  let rd0 = ext_method!(
    Some("en".to_string()); None; true;
    (io.raddr) -> () {}
  );

  let rd1 = ext_method!(
    None; Some("rd1_valid".to_string()); false;
    () -> (io.rdata) {}
  );

  let write = ext_method!(
    Some("wen".to_string()); None; true;
    (io.wdata, io.waddr) -> () {}
  );

  method_rel!(write C write);
  method_rel!(rd0 C rd0);
  schedule!(rd1, rd0, write);
}

itfc_declare! {
  /// Data type of the memory
  param DT;
  /// Address type of the memory, should be able to hold n^2 addresses
  param AT;

  /// A 2 dimensional [`Mem1r1w`]
  pub struct Mem1r1w2d {
    wdata: input param DT,
    raddr0: input param AT,
    raddr1: input param AT,
    waddr0: input param AT,
    waddr1: input param AT,
    rdata: output param DT,
  }

  /// Start read data at [raddr0, raddr1]
  method rd0(raddr0, raddr1);
  /// Get the result of read
  method rd1() -> (rdata);
  /// Write data into [waddr0, waddr1]
  method write(wdata, waddr0, waddr1);
}

/// Make a 2-D mem1r1w
#[module]
pub fn mem1r1w2d(dt: &Type, at: &Type, n: usize, m: usize) -> Mem1r1w2d {
  let io = io! {DT: dt, AT: at};
  distinguisher!(&format!("{n}_{m}"));
  anno!("synthesis": "true");

  let mem = instance!(mem1r1w(dt, at, n * m));

  let rd0 = method!(
    (io.raddr0, io.raddr1) {
      mem.rd0(io.raddr0 * literal(m as i32, at) + io.raddr1);
    }
  );
  let rd1 = method!(
    () -> (io.rdata) {
      mem.rd1()
    }
  );
  let write = method!(
    (io.wdata, io.waddr0, io.waddr1) {
      mem.write(io.wdata, io.waddr0 * literal(m as i32, at) + io.waddr1);
    }
  );

  method_rel!(write C write);
  method_rel!(rd0 C rd0);
  schedule!(rd1, rd0, write);
}

itfc_declare! {
  /// Data Type
  param DT;
  /// Address Type
  param AT;

  pub struct Mem1r1wxk {
  }
}

/// Make k [`Mem1r1w`] for memory partition
#[module]
pub fn mem1r1wxk(dt: &Type, at: &Type, m: usize, k: usize) -> Mem1r1wxk {
  io! {DT: dt, AT: at};
  distinguisher!(&format!("{m}_{k}"));
  anno!("synthesis": "true");
  for i in 0..k {
    let wdata = input!(format!("wdata_{i}"), dt.clone());
    let waddr = input!(format!("waddr_{i}"), at.clone());
    let raddr = input!(format!("raddr_{i}"), at.clone());
    let rdata = output!(format!("rdata_{i}"), dt.clone());
    let mem = named_instance!(format!("mem_{i}"); mem1r1w(dt, at, m));
    let rd0 = named_method! {
      format!("rd0_{i}");
      (raddr) {
        mem.rd0(raddr);
      }
    };
    let rd1 = named_method! {
      format!("rd1_{i}");

      () -> (rdata) {
        mem.rd1()
      }
    };
    let write = named_method! {
      format!("write_{i}");
      (wdata, waddr) {
        mem.write(wdata, waddr);
      }
    };

    method_rel!(write C write);
    method_rel!(rd0 C rd0);
    schedule!(rd1, rd0, write);
  }
}

#[allow(dead_code)]
impl Mem1r1wxk {
  /// Start reading from the i-th port
  pub fn rd0(&self, i: usize, raddr: impl CmtAST) {
    self.__ctx_view.invoke(format!("rd0_{i}"), vec![raddr.ast()], 0, None);
  }

  /// Get data from the last read at the i-th port
  pub fn rd1(&self, i: usize) -> impl CmtAST {
    self.__ctx_view.invoke(format!("rd1_{i}"), vec![], 1, None).into_iter().next().unwrap()
  }

  /// Write data to the i-th port
  pub fn write(&self, i: usize, wdata: impl CmtAST, waddr: impl CmtAST) {
    self.__ctx_view.invoke(format!("write_{i}"), vec![wdata.ast(), waddr.ast()], 0, None);
  }
}

itfc_declare! {
  /// Data type
  param DT;
  /// Address type
  param AT;

  pub struct Mem1r1w2dxk {
  }
}

/// Make k [`Mem1r1w2d`] for memory partition
#[module]
pub fn mem1r1w2dxk(dt: &Type, at: &Type, m: usize, n: usize, k: usize) -> Mem1r1w2dxk {
  io! {DT: dt, AT: at};
  distinguisher!(&format!("{m}_{n}_{k}"));
  anno!("synthesis": "true");
  for i in 0..k {
    let wdata = input!(format!("wdata_{i}"), dt.clone());
    let waddr_x = input!(format!("waddr_x_{i}"), at.clone());
    let waddr_y = input!(format!("waddr_y_{i}"), at.clone());
    let raddr_x = input!(format!("raddr_x_{i}"), at.clone());
    let raddr_y = input!(format!("raddr_y_{i}"), at.clone());
    let rdata = output!(format!("rdata_{i}"), dt.clone());
    let mem = named_instance!(format!("mem_{i}"); mem1r1w2d(dt, at, m, n));
    let rd0 = named_method! {
      format!("rd0_{i}");
      (raddr_x, raddr_y) {
        mem.rd0(raddr_x, raddr_y);
      }
    };
    let rd1 = named_method! {
      format!("rd1_{i}");

      () -> (rdata) {
        mem.rd1()
      }
    };
    let write = named_method! {
      format!("write_{i}");
      (wdata, waddr_x, waddr_y) {
        mem.write(wdata, waddr_x, waddr_y);
      }
    };

    method_rel!(write C write);
    method_rel!(rd0 C rd0);
    schedule!(rd1, rd0, write);
  }
}

#[allow(dead_code)]
impl Mem1r1w2dxk {
  /// Start reading from the i-th port
  pub fn rd0(&self, i: usize, raddr_x: impl CmtAST, raddr_y: impl CmtAST) {
    self.__ctx_view.invoke(format!("rd0_{i}"), vec![raddr_x.ast(), raddr_y.ast()], 0, None);
  }

  /// Get data from the last read at the i-th port
  pub fn rd1(&self, i: usize) -> impl CmtAST {
    self.__ctx_view.invoke(format!("rd1_{i}"), vec![], 1, None).into_iter().next().unwrap()
  }

  /// Write data to the i-th port
  pub fn write(&self, i: usize, wdata: impl CmtAST, waddr_x: impl CmtAST, waddr_y: impl CmtAST) {
    self.__ctx_view.invoke(format!("write_{i}"), vec![wdata.ast(), waddr_x.ast(), waddr_y.ast()], 0, None);
  }
}

itfc_declare! {
  /// Data type
  param DT;
  /// Address type
  param AT;

  /// A 1r1w memory with combinational read and synchronous write
  pub struct Memcrsw {
    wdata: input param DT,
    raddr: input param AT,
    waddr: input param AT,
    rdata: output param DT,
  }

  /// Read data at raddr and get the result at the same cycle
  method read(raddr) -> (rdata);
  /// Write data to the memory
  method write(wdata, waddr) -> ();
}

/// Memory, combinational read, sync write
///
/// i.e. read latency = 0, write latency = 1
#[module]
pub fn memcrsw(dt: &Type, at: &Type, depth: usize) -> Memcrsw {
  let io = io! {DT: dt, AT: at};
  anno!("synthesis": "true");
  distinguisher!(&format!("{depth}"));
  let fir_str = r#"
module Memcrsw_{ctype D}_{ctype A}_{const N}:
    input clk: Clock
    input en: UInt<1>
    input raddr: {type A}
    output rdata: {type D}
    input wen: UInt<1>
    input waddr: {type A}
    input wdata: {type D}

    mem mymemory:
        data-type => {type D}
        depth => {const N}
        read-latency => 0
        write-latency => 1
        reader => r
        writer => w
        read-under-write => old

    mymemory.r.en <= en
    mymemory.r.clk <= clk
    mymemory.r.addr <= raddr
    rdata <= mymemory.r.data

    mymemory.w.en <= wen
    mymemory.w.clk <= clk
    mymemory.w.addr <= waddr
    mymemory.w.data <= wdata
    mymemory.w.mask <= UInt<1>(1)
"#;
  let dt: ir::Type = dt.clone().into();
  let at: ir::Type = at.clone().into();
  let fir_str = fir_str.replace("{ctype D}", &dt.ir_dump());
  let fir_str = fir_str.replace("{ctype A}", &at.ir_dump());
  let fir_str = fir_str.replace("{const N}", &depth.to_string());
  let fir_str = fir_str.replace("{type D}", &cmtir_type_to_firrtl_type(dt).to_string());
  let fir_str = fir_str.replace("{type A}", &cmtir_type_to_firrtl_type(at).to_string());

  external!(
    ["wdata".to_string(), "raddr".to_string(), "waddr".to_string()],
    ["rdata".to_string()],
    Some("clk".to_string()),
    None,
    fir_str.to_string()
  );

  let read = ext_method!(
    Some("en".to_string()); None; false;
    (io.raddr) -> (io.rdata) {}
  );

  let write = ext_method!(
    Some("wen".to_string()); None; true;
    (io.wdata, io.waddr) -> () {}
  );

  method_rel!(write C write);
  method_rel!(read C read);
  schedule!(read, write);
}
