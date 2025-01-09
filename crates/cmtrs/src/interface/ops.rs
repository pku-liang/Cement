//! Operators for Cmtrs values, most are from FIRRTL
//!
//! Currently supported operators:
//! + Rust ops
//! + [`DShl::dshl`], dynamic shift left, where the rhs is a runtime value
//! + [`DShr::dshr`], dynamic shift right, where the rhs is a runtime value
//! + [`Cat::cat`], bit concatenation
//! + [`Cvt::cvt`], convert type into UInt
//! + [`AsUInt::as_uint`], convert type into UInt
//! + [`AsSInt::as_sint`], convert type into SInt
//! + [`Andr::andr`], reduce and
//! + [`Orr::orr`], reduce or
//! + [`Xorr::xorr`], reduce xor
//! + [`Repeat::repeat`], repeat a value for constant times
//! + [`Pad::pad`], pad zeros
//! + [`Head::head`], get MSBs
//! + [`Tail::tail`], get LSBs
//! + [`ExtractBits::bits`], get a range of bits

pub use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Neg, Not, Rem, Shl, Shr, Sub};

use super::*;
use crate::stl;

pub trait DShl<RHS> {
  type Output;
  fn dshl(self, rhs: RHS) -> Self::Output;
}

pub trait DShr<RHS> {
  type Output;
  fn dshr(self, rhs: RHS) -> Self::Output;
}

pub trait Cat<RHS> {
  type Output;
  fn cat(self, rhs: RHS) -> Self::Output;
}

macro_rules! impl_binary {
  ($trait_name: ident, $fn_name: ident, $prim: ident) => {
    impl_binary!($trait_name, $fn_name, $prim, (&Var, Var, Vars, &stl::Wire, &stl::Reg, &stl::Integer));
  };
  ($trait_name: ident, $fn_name: ident, $prim: ident, ($($t: ty),*)) => {
    $(
      impl_binary!($trait_name, $fn_name, $prim, $t);
    )*
  };
  ($trait_name: ident, $fn_name: ident, $prim: ident, $t: ty) => {
    impl<T> $trait_name<T> for $t
    where T: CmtAST
    {
      type Output = Var;

      #[track_caller]
      fn $fn_name(self, rhs: T) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::binary(Prim::$prim, self.ast(), rhs.ast(), Some(span)))
      }
    }
  };
}

impl_binary!(Add, add, Add);
impl_binary!(Sub, sub, Sub);
impl_binary!(Mul, mul, Mul);
impl_binary!(Div, div, Div);
impl_binary!(Rem, rem, Rem);
impl_binary!(BitAnd, bitand, And);
impl_binary!(BitOr, bitor, Or);
impl_binary!(BitXor, bitxor, Xor);
impl_binary!(DShl, dshl, DShl);
impl_binary!(DShr, dshr, DShr);
impl_binary!(Cat, cat, Cat);

macro_rules! impl_unary {
  ($trait_name: ident, $fn_name: ident, $prim: ident) => {
    impl_unary!($trait_name, $fn_name, $prim, &Var, Var, Vars, &stl::Wire, &stl::Reg, &stl::Integer);
  };
  ($trait_name: ident, $fn_name: ident, $prim: ident, $($t: ty),*) => {
    $(
      impl $trait_name for $t {
        type Output = Var;

        #[track_caller]
        fn $fn_name(self) -> Self::Output {
          let span = extract_span_from_location(Location::caller());
          Var::new(AST::unary(Prim::$prim, self.ast(), Some(span)))
        }
      }
    )*
  };
}

pub trait Cvt {
  type Output;
  fn cvt(self) -> Self::Output;
}

pub trait AsUInt {
  type Output;
  fn as_uint(self) -> Self::Output;
}

pub trait AsSInt {
  type Output;
  fn as_sint(self) -> Self::Output;
}

pub trait Andr {
  type Output;
  fn andr(self) -> Self::Output;
}

pub trait Orr {
  type Output;
  fn orr(self) -> Self::Output;
}

pub trait Xorr {
  type Output;
  fn xorr(self) -> Self::Output;
}

impl_unary!(Neg, neg, Neg);
impl_unary!(Not, not, Not);
impl_unary!(Cvt, cvt, Cvt);
impl_unary!(AsUInt, as_uint, AsUInt);
impl_unary!(AsSInt, as_sint, AsSInt);
impl_unary!(Andr, andr, Andr);
impl_unary!(Orr, orr, Orr);
impl_unary!(Xorr, xorr, Xorr);

macro_rules! impl_binary_w_attr {
  ($trait_name: ident, $fn_name: ident, $prim: ident) => {
    impl_binary_w_attr!($trait_name, $fn_name, $prim, &Var, Var, Vars, &stl::Wire, &stl::Reg, &stl::Integer);
  };
  ($trait_name: ident, $fn_name: ident, $prim: ident, $($t: ty),*) => {
    $(
      impl $trait_name<u32> for $t {
        type Output = Var;

        #[track_caller]
        fn $fn_name(self, rhs: u32) -> Self::Output {
          let span = extract_span_from_location(Location::caller());
          Var::new(AST::binary_w_attr(Prim::$prim, self.ast(), rhs, Some(span)))
        }
      }
    )*
  };
}

pub trait Repeat<RHS> {
  type Output;
  fn repeat(self, cnt: RHS) -> Self::Output;
}

pub trait Pad<RHS> {
  type Output;
  fn pad(self, rhs: RHS) -> Self::Output;
}

pub trait Head<RHS> {
  type Output;
  fn head(self, rhs: RHS) -> Self::Output;
}

pub trait Tail<RHS> {
  type Output;
  fn tail(self, rhs: RHS) -> Self::Output;
}

impl_binary_w_attr!(Shl, shl, Shl);
impl_binary_w_attr!(Shr, shr, Shr);
impl_binary_w_attr!(Pad, pad, Pad);
impl_binary_w_attr!(Head, head, Head);
impl_binary_w_attr!(Tail, tail, Tail);

pub trait CmpOps {
  type Output;

  fn eq(self, rhs: impl CmtAST) -> Self::Output;
  fn ne(self, rhs: impl CmtAST) -> Self::Output;
  fn lt(self, rhs: impl CmtAST) -> Self::Output;
  fn le(self, rhs: impl CmtAST) -> Self::Output;
  fn gt(self, rhs: impl CmtAST) -> Self::Output;
  fn ge(self, rhs: impl CmtAST) -> Self::Output;
}

macro_rules! impl_cmp_ops {
  ($ty: ty) => {
    impl CmpOps for $ty {
      type Output = Var;

      fn eq(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Eq, self.ast(), rhs.ast(), Some(span)))
      }

      fn ne(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Neq, self.ast(), rhs.ast(), Some(span)))
      }

      fn lt(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Lt, self.ast(), rhs.ast(), Some(span)))
      }

      fn le(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Leq, self.ast(), rhs.ast(), Some(span)))
      }

      fn gt(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Gt, self.ast(), rhs.ast(), Some(span)))
      }

      fn ge(self, rhs: impl CmtAST) -> Self::Output {
        let span = extract_span_from_location(Location::caller());
        Var::new(AST::cmp(Cmp::Geq, self.ast(), rhs.ast(), Some(span)))
      }
    }
  };
}
impl_cmp_ops!(&Var);
impl_cmp_ops!(Var);
impl_cmp_ops!(Vars);
impl_cmp_ops!(&stl::Wire);
impl_cmp_ops!(&stl::Reg);
impl_cmp_ops!(&stl::Integer);

pub trait ExtractBits<A1, A2> {
  type Output;
  fn bits(self, a1: A1, a2: A2) -> Self::Output;
}

macro_rules! impl_binary_w_2attr {
  ($trait_name: ident, $fn_name: ident, $prim: ident) => {
    impl_binary_w_2attr!($trait_name, $fn_name, $prim, &Var, Var, Vars, &stl::Wire, &stl::Reg, &stl::Integer);
  };
  ($trait_name: ident, $fn_name: ident, $prim: ident, $($t: ty),*) => {
    $(
      impl $trait_name<u32, u32> for $t {
        type Output = Var;

        fn $fn_name(self, a1: u32, a2: u32) -> Self::Output {
          let span = extract_span_from_location(Location::caller());
          Var::new(AST::binary_w_2attr(Prim::$prim, self.ast(), a1, a2, Some(span)))
        }
      }
    )*
  };
}

impl_binary_w_2attr!(ExtractBits, bits, Bits);
