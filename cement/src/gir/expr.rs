use std::hash::Hash;

use irony_cmt::{CombICmpPredicate, CombUnaryPredicate, CombVariadicPredicate};

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub enum UnaryOpType {
  Neg,
  Not,
}

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub enum BinaryOpType {
  Add,
  //   Sub,
  And,
  Or,
  Xor,
  //   Cat,
  //   Repeat,
  Eq,
  Neq,
  Lt,
  Le,
  Gt,
  Ge,
}

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub enum ReduceOpType {
  Sum,
  And,
  Or,
  //   Cat,
}

pub fn reduce2binary_op(op: ReduceOpType) -> BinaryOpType {
  match op {
    ReduceOpType::And => BinaryOpType::And,
    ReduceOpType::Or => BinaryOpType::Or,
    // ReduceOpType::Cat => BinaryOpType::Cat,
    ReduceOpType::Sum => BinaryOpType::Add,
  }
}

pub fn binary2variadic_op(op: BinaryOpType) -> Option<CombVariadicPredicate> {
  match op {
    BinaryOpType::Add => Some(CombVariadicPredicate::Add),
    BinaryOpType::And => Some(CombVariadicPredicate::And),
    BinaryOpType::Or => Some(CombVariadicPredicate::Or),
    BinaryOpType::Xor => Some(CombVariadicPredicate::Xor),
    _ => None,
  }
}

pub fn binary2icmp_op(op: BinaryOpType) -> Option<CombICmpPredicate> {
  match op {
    BinaryOpType::Eq => Some(CombICmpPredicate::CEQ),
    BinaryOpType::Neq => Some(CombICmpPredicate::CNE),
    BinaryOpType::Le => Some(CombICmpPredicate::ULE),
    BinaryOpType::Lt => Some(CombICmpPredicate::ULT),
    BinaryOpType::Ge => Some(CombICmpPredicate::UGE),
    BinaryOpType::Gt => Some(CombICmpPredicate::UGT),
    _ => None,
  }
}

pub fn unary2cmtc_op(op: UnaryOpType) -> CombUnaryPredicate {
  match op {
    UnaryOpType::Neg => CombUnaryPredicate::Neg,
    UnaryOpType::Not => CombUnaryPredicate::Not,
  }
}
