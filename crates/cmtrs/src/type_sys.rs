//! Cmtrs Type System
//!
//! Currently Type::UInt / Type::SInt and Integer are well-supported, while the others are working in progress.

use cmtir as ir;

// TODO: support more types
#[derive(Debug, Clone)]
pub enum Type {
  /// software integer type, (rust i32)
  Integer,
  /// unsigned int with width
  UInt(u32),
  /// signed int with width
  SInt(u32),
  /// probe, rwprobe, NOT HARDWARE
  Ref(u32),
  /// vector type
  Vector(Box<Type>, u32),
  /// bundle type
  Struct(Vec<(String, Type, bool)>),
  /// union type
  Enum(Vec<(String, Option<Type>)>),
}

impl Type {
  pub fn name(&self) -> String {
    let ir_type: ir::Type = self.clone().into();
    ir_type.to_string()
  }
}

impl Into<ir::Type> for Type {
  fn into(self) -> ir::Type {
    match self {
      Type::Integer => ir::Type::Integer,
      Type::UInt(x) => ir::Type::uint(x),
      Type::SInt(x) => ir::Type::sint(x),
      Type::Ref(x) => ir::Type::new_ref(x),
      Type::Vector(x, y) => ir::Type::vector((*x).into(), y),
      Type::Struct(x) => ir::Type::bundle(x.into_iter().map(|(name, ty, flip)| (name, ty.into(), flip)).collect()),
      Type::Enum(x) => ir::Type::union(x.into_iter().map(|(name, ty)| (name, ty.map(|ty| ty.into()))).collect()),
    }
  }
}

impl From<ir::Type> for Type {
  fn from(ty: ir::Type) -> Self {
    match ty {
      ir::Type::Integer => Self::Integer,
      ir::Type::UInt(x) => Self::UInt(x),
      ir::Type::SInt(x) => Self::SInt(x),
      ir::Type::Ref(x) => Self::Ref(x),
      ir::Type::Vector(x, y) => Self::Vector(Box::new((*x).into()), y),
      ir::Type::Bundle(x) => Self::Struct(x.into_iter().map(|(name, ty, flip)| (name, ty.into(), flip)).collect()),
      ir::Type::Enum(x) => Self::Enum(x.into_iter().map(|(name, ty)| (name, Some(ty.into()))).collect()),
    }
  }
}

pub trait TypeChecker {
  fn accept(ty: &Type) -> bool;
}
