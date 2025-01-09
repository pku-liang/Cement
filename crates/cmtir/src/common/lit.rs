use super::*;

pub use num::{bigint::Sign, BigInt, ToPrimitive};

/// A radix of an integer literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SExpr)]
pub enum Radix {
  B,
  O,
  H,
}

impl Radix {
  /// Convert the radix to a u32.
  pub fn to_u32(&self) -> u32 {
    match self {
      Radix::B => 2,
      Radix::O => 8,
      Radix::H => 16,
    }
  }
  /// Convert the radix to a string.
  pub fn to_str(&self) -> &str {
    match self {
      Radix::B => "b",
      Radix::O => "o",
      Radix::H => "h",
    }
  }
  /// Convert the character to a radix.
  pub fn from_char(c: char) -> Result<Self, String> {
    match c {
      'b' => Ok(Radix::B),
      'o' => Ok(Radix::O),
      'h' => Ok(Radix::H),
      _ => Err(format!("Invalid radix: {}", c)),
    }
  }
}

/// A radix integer literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RadixIntLit {
  sign: Sign,
  radix: Radix,
  digits: Vec<u8>,
}

impl RadixIntLit {
  /// Check if the literal is negative.
  pub fn is_neg(&self) -> bool {
    self.sign == Sign::Minus
  }

  /// Check if the literal is true.
  pub fn is_true(&self) -> bool {
    self.digits.iter().any(|d| *d != 0)
  }

  /// Create a radix integer literal from a big integer.
  pub fn from_bigint(value: BigInt) -> Result<Self, String> {
    Ok(RadixIntLit {
      sign: value.sign(),
      radix: Radix::B,
      digits: bigint_to_binary_digits(value)?,
    })
  }

  /// Create a radix integer literal from a string.
  pub fn from_str(s: &str) -> Result<Self, String> {
    let mut parser = Parser::new(s);
    parser.parse()
  }

  /// Convert the literal to a decimal number.
  pub fn to_demical(&self) -> BigInt {
    let mut res = BigInt::from(0);
    for (i, digit) in self.digits.iter().enumerate() {
      res +=
        BigInt::from(*digit) * BigInt::from(self.radix.to_u32()).pow(i as u32);
    }
    res
  }

  /// Convert the literal to a string.
  pub fn to_string(&self) -> String {
    let mut s = format!(
      "{}0{}",
      if self.sign == Sign::Minus { "-" } else { "" },
      self.radix.to_str()
    );
    for digit in self.digits.iter().rev() {
      s.push(
        std::char::from_digit(*digit as u32, self.radix.to_u32()).unwrap(),
      );
    }
    s
  }

  /// Convert the literal to a binary digits.
  pub fn to_binary_digits(&self) -> Vec<bool> {
    self
      .digits
      .iter()
      .flat_map(|d| match self.radix {
        Radix::B => vec![*d == 1],
        Radix::O => {
          let mut res = Vec::new();
          for i in 0..3 {
            res.push(*d & (1 << i) != 0);
          }
          res
        }
        Radix::H => {
          let mut res = Vec::new();
          for i in 0..4 {
            res.push(*d & (1 << i) != 0);
          }
          res
        }
      })
      .collect()
  }

  /// Calculate the length of the binary representation of the literal.
  pub fn binary_len(&self) -> usize {
    self.digits.len()
      * match self.radix {
        Radix::B => 1,
        Radix::O => 3,
        Radix::H => 4,
      }
  }
}

impl Parse for RadixIntLit {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    let is_neg = if parser.peek_any(&[Token::Number]) {
      let is_neg = parser.peek()?.0.starts_with("-");
      let s: BigInt = parser.parse()?;
      if s != BigInt::from(0)
        || !parser.peek_fn(|s, _t| {
          s.starts_with('b') || s.starts_with('o') || s.starts_with('h')
        })
      {
        return RadixIntLit::from_bigint(s);
      }
      is_neg
    } else {
      return Err(
        "Invalid integer literal, not starting with any number".to_string(),
      );
    };

    // 0[b/o/h]xxxx, 0 has been parsed above
    let (s, _) = parser.next()?;
    let radix = Radix::from_char(s.chars().next().unwrap())?;
    let digits = str_to_digits(&s[1..], radix)?;

    Ok(RadixIntLit {
      sign: if is_neg { Sign::Minus } else { Sign::NoSign },
      radix,
      digits,
    })
  }
}

impl Print for RadixIntLit {
  fn print<'p>(&'p self, printer: &mut Printer<'p>) {
    let mut s = format!(
      "{}0{}",
      if self.sign == Sign::Minus { "-" } else { "" },
      self.radix.to_str()
    );
    for digit in self.digits.iter().rev() {
      s.push(
        std::char::from_digit(*digit as u32, self.radix.to_u32()).unwrap(),
      );
    }
    printer.write_fmt(format_args!("{}", s));
  }
}

/// Implement `From` for `RadixIntLit` from unsigned integers.
macro_rules! impl_from_uint_for_intlit {
    ($($t:ty),*) => {
        $(
            impl From<$t> for RadixIntLit {
                fn from(value: $t) -> Self {
                    let mut value = value;
                    let mut digits = Vec::new();
                    for _ in 0..(<$t>::BITS) {
                        digits.push((value & 1) as u8);
                        value >>= 1;
                    }
                    RadixIntLit {
                        sign: Sign::Plus,
                        radix: Radix::B,
                        digits,
                    }
                }
            }
        )*
    };
}

impl_from_uint_for_intlit!(u8, u16, u32, u64, u128, usize);

/// Implement `From` for `RadixIntLit` from signed integers.
macro_rules! impl_from_int_for_intlit {
    ($($t:ty),*) => {
        $(
            impl From<$t> for RadixIntLit {
                fn from(value: $t) -> Self {
                  RadixIntLit::from_bigint(BigInt::from(value)).unwrap()
                }
            }
        )*
    };
}

impl_from_int_for_intlit!(i8, i16, i32, i64, i128, isize);

/// Convert a big integer to a binary digits.
pub fn bigint_to_binary_digits(value: BigInt) -> Result<Vec<u8>, String> {
  let mut digits = Vec::new();
  let mut v = value;
  if v.sign() == Sign::Minus {
    v = -v;
  }
  let zero = BigInt::from(0);
  let two = BigInt::from(2);
  while v > zero {
    digits.push((&v % &two).to_u8().ok_or(format!("Integer overflow"))?);
    v /= &two;
  }
  if digits.is_empty() {
    digits = vec![0];
  }
  Ok(digits)
}

pub fn str_to_digits(s: &str, radix: Radix) -> Result<Vec<u8>, String> {
  let mut digits = Vec::new();
  for c in s.chars().rev() {
    digits.push(
      c.to_digit(radix.to_u32())
        .ok_or(format!("Invalid digit: {}", c))? as u8,
    );
  }
  Ok(digits)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_radix_int_lit() {
    let lit = RadixIntLit::from_str("0b1010").unwrap();
    assert_eq!(lit.to_string(), "0b1010");
    assert_eq!(lit.is_neg(), false);
    assert_eq!(lit.is_true(), true);
  }

  #[test]
  fn test_neg_radix_int_lit() {
    let lit = RadixIntLit::from_str("-0b1010").unwrap();
    assert_eq!(lit.to_string(), "-0b1010");
    assert_eq!(lit.is_neg(), true);
    assert_eq!(lit.is_true(), true);
  }

  #[test]
  fn test_neg_i8() {
    let lit = RadixIntLit::from(-42_i8);
    assert_eq!(lit.to_string(), "-0b101010");
    assert_eq!(lit.is_neg(), true);
    assert_eq!(lit.is_true(), true);
  }
}
