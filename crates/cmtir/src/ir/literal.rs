//! Literal operations in `cmtir`.
use super::*;

/// A literal with type.
#[derive(Debug, Clone, SExpr)]
pub struct TypedLit {
  #[pp(open)]
  pub lit: RadixIntLit,
  #[pp(close)]
  pub ty: Type,
}

impl TypedLit {
  /// Create a new typed literal.
  pub fn new(lit: RadixIntLit, ty: Type) -> Self {
    Self { lit, ty }
  }
  /// Check if the literal is true.
  pub fn is_true(&self) -> bool {
    self.lit.is_true()
  }
}

/// Implement `From` for `TypedLit` from unsigned integers.
macro_rules! impl_from_uint_for_typedlit {
    ($(($t:ty, $width:expr)),*) => {
        $(
            impl From<$t> for TypedLit {
                fn from(value: $t) -> Self {
                    TypedLit::new(value.into(), Type::UInt($width))
                }
            }
        )*
    };
}

impl_from_uint_for_typedlit!(
  (u8, 8),
  (u16, 16),
  (u32, 32),
  (u64, 64),
  (u128, 128),
  (usize, usize::BITS)
);

/// Implement `From` for `TypedLit` from signed integers.
macro_rules! impl_from_int_for_typedlit {
  ($(($t:ty, $width:expr)),*) => {
      $(
          impl From<$t> for TypedLit {
              fn from(value: $t) -> Self {
                  TypedLit::new(value.into(), Type::SInt($width))
              }
          }
      )*
  };
}

impl_from_int_for_typedlit!(
  (i8, 8),
  (i16, 16),
  (i32, 32),
  (i64, 64),
  (i128, 128),
  (isize, isize::BITS)
);

impl TypedLit {
  /// Convert the literal to FIRRTL literal.
  pub fn to_fir_lit(&self) -> fir::Expression {
    fir::Expression::literal(
      match self.ty {
        Type::UInt(_) => fir::Sign::Unsigned,
        Type::SInt(_) => fir::Sign::Signed,
        _ => unreachable!(),
      },
      self.lit.is_neg(),
      radix_int_lit_to_logic_values(self.lit.clone()),
      Some(self.ty.int_width() as usize),
    )
  }
}

/// Convert the literal to FIRRTL logic values.
pub fn radix_int_lit_to_logic_values(lit: RadixIntLit) -> fir::LogicValues {
  let mut logic_values = Vec::new();
  let mut bits = lit.to_binary_digits();
  if bits.is_empty() {
    bits = vec![false];
  }
  // log::error!("bits: {:?}", bits);
  for bit in bits.iter().rev() {
    logic_values.push(fir::LogicValue::from_bool(*bit));
  }

  // log::error!("logic values: {:?}", logic_values);

  fir::LogicValues::new(logic_values)
}
