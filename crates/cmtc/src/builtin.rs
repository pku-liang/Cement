#![allow(unused)]

use cmtir as ir;
use cmtir::to_fir::cmtir_type_to_firrtl_type;
use cmtir::{IRDump, Parser};

use cmtrs::{stl, CmtLit, Instance};

pub fn reg_gen(ty: ir::Type) -> ir::Module {
  let mut reg = stl::reg(&ty.into());
  let mut module = reg.to_cmtir().modules.into_iter().next().unwrap();
  module.unset_top();
  module
}

pub fn wire_gen(ty: ir::Type) -> ir::Module {
  let mut wire = stl::wire(&ty.into());
  let mut module = wire.to_cmtir().modules.into_iter().next().unwrap();
  module.unset_top();
  module
}

pub fn fifo1_gen(ty: ir::Type) -> Vec<ir::Module> {
  let mut fifo1 = stl::fifo1_push(&ty.into());
  fifo1
    .to_cmtir()
    .modules
    .into_iter()
    .map(|mut m| {
      m.unset_top();
      m
    })
    .collect()
}

pub fn shift_reg_gen(ty: ir::Type, len: usize) -> Vec<ir::Module> {
  let mut shift_reg = stl::shift_reg(&ty.clone().into(), len, 0);
  shift_reg
    .to_cmtir()
    .modules
    .into_iter()
    .map(|mut m| {
      m.unset_top();
      m
    })
    .collect()
}

pub fn mem_1r1w_gen(ty: ir::Type, addr_ty: ir::Type, len: usize) -> ir::Module {
  let mut mem_1r1w =
    stl::mem1r1w(&ty.clone().into(), &addr_ty.clone().into(), len);
  let mut module = mem_1r1w.to_cmtir().modules.into_iter().next().unwrap();
  module.unset_top();
  module
}

#[cfg(test)]
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_wire_gen() {
    let wire = wire_gen(ir::Type::UInt(4));
    println!("{}", wire.ir_dump());
  }

  #[test]
  fn test_shift_reg_gen() {
    let shift_reg = shift_reg_gen(ir::Type::UInt(4), 4);
    for module in shift_reg {
      println!("{}", module.ir_dump());
    }
  }
}
