use std::collections::HashSet;
use std::hash::Hash;

use arrayvec::ArrayVec;
use bitvec::prelude as bv;
use rithm::big_int;
use traiter::numbers::{Endianness, FromBytes, Pow, Signed, ToBytes};

#[cfg(target_arch = "x86")]
type Digit = u16;
#[cfg(not(target_arch = "x86"))]
type Digit = u32;
const DIGIT_BITNESS: usize = (Digit::BITS - 1) as usize;
const _: () = assert!(big_int::is_valid_digit_bitness::<Digit, DIGIT_BITNESS>());
pub type BigInt = big_int::BigInt<Digit, DIGIT_BITNESS>;

pub type BitVec = bv::BitVec<u8, bv::Lsb0>;
pub type BitSlice = bv::BitSlice<u8, bv::Lsb0>;

// #[derive(Clone, Copy, Debug)]
// pub struct Bits<const N: usize>([bool; N]);

pub fn bits_str(bits: Vec<bool>) -> String {
  let mut s = String::new();
  for bit in bits.iter().rev() {
    s.push(if *bit { '1' } else { '0' });
  }
  s
}

/// Returns bit-represented value of an integer.
pub fn usize_to_bitvec(n: usize, value: usize) -> Vec<bool> {
  assert!(
    n >= clog2(value + 1),
    "Width of Expr ({}) is too small to be converted from the value '{}'",
    n,
    value
  );
  let size_of_usize = ::std::mem::size_of::<usize>();
  (0..n)
    .map(|i| if i >= size_of_usize * 8 { false } else { (value & (1 << i)) != 0 })
    .collect::<Vec<_>>()
}

pub fn _from_bits_to_u32(bits: Vec<bool>) -> u32 {
  let mut sum: u32 = 0;
  for (i, bit) in bits.iter().enumerate() {
    sum += (*bit as u32) << (i as u32);
  }
  sum
}

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn _usize_to_bits<const N: usize>(value: usize) -> [bool; N] {
  usize_to_bitvec(N, value).try_into().unwrap()
}

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn _u32_to_bits<const N: usize>(value: u32) -> [bool; N] {
  let size_of_u32 = ::std::mem::size_of::<u32>();
  (0..N)
    .map(|i| if i >= size_of_u32 * 8 { false } else { (value & (1 << i)) != 0 })
    .collect::<ArrayVec<bool, N>>()
    .into_inner()
    .unwrap()
}

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn _u64_to_bits<const N: usize>(value: u64) -> [bool; N] {
  let size_of_u64 = ::std::mem::size_of::<u64>();
  (0..N)
    .map(|i| if i >= size_of_u64 * 8 { false } else { (value & (1 << i)) != 0 })
    .collect::<ArrayVec<bool, N>>()
    .into_inner()
    .unwrap()
}

/// Returns ceiling log2.
pub const fn clog2(value: usize) -> usize {
  if value == 0 {
    0
  } else if value == 1 {
    1
  } else {
    (::std::mem::size_of::<usize>() * 8) - (value - 1).leading_zeros() as usize
  }
}

/// Returns floor log2
pub const fn _flog2(val: usize) -> usize {
  if val == 1 {
    0
  } else {
    1 + _flog2(val >> 1)
  }
}

pub fn _option_to_vec<T>(opt: Option<T>) -> Vec<T> {
  match opt {
    Some(x) => vec![x],
    None => Vec::new(),
  }
}

pub fn hashset_from_one<T: Eq + Hash>(ele: T) -> HashSet<T> {
  let mut s = HashSet::new();
  s.insert(ele);
  s
}
pub fn hashset_only_element<T: Eq + Hash>(set: &HashSet<T>, ele: &T) -> bool {
  set.len() == 1 && set.contains(ele)
}

pub fn hashset_merge<T: Eq + Hash + Copy>(
  s1: &HashSet<T>, s2: &HashSet<T>,
) -> HashSet<T> {
  s1.union(s2).copied().collect()
}

pub fn hashset_addone<T: Eq + Hash + Copy>(mut s1: HashSet<T>, ele: T) -> HashSet<T> {
  s1.insert(ele);
  s1
}

// pub fn bigint_2comp(x: &BigInt, w: usize) -> BigInt {
//   if x.is_positive() {
//     x.clone()
//   } else {
//     x + BigInt::from(2).pow(BigInt::from(w - 1))
//   }
// }

// pub fn bits2bigint(x: &[bool], signed: bool) -> BigInt {
//   let mut bytes = Vec::new();
//   let n = x.len();

//   let mut cnt = 0;
//   let mut a = 0u8;
//   if n % 8 != 0 {
//     for _ in 0..(8 - n % 8) {
//       a = (a << 1) | if signed { 1 } else { 0 };
//     }
//     cnt = 8 - n % 8;
//   }
//   for num in x.iter().rev() {
//     if *num {
//       a = (a << 1) | 1;
//     } else {
//       a <<= 1;
//     }
//     cnt += 1;
//     if cnt == 8 {
//       cnt = 0;
//       bytes.push(a);
//       a = 0;
//     }
//   }

//   let result = BigInt::from_bytes(&bytes, Endianness::Big);
//   result
// }

// pub fn bitVec2BigInt(x: &BitVec, signed: bool) -> BigInt {
//   if !signed {
//     let mut y = bv::bitvec![u8, bv::Msb0; 0; 8];
//     y.extend_from_bitslice(x);
//     BigInt::from_bytes(y.as_raw_slice(), Endianness::Big)
//   } else {
//     BigInt::from_bytes(x.as_raw_slice(), Endianness::Big)
//   }
// }

// pub fn bigInt2BitVec(x: &BigInt, signed: bool) -> BitVec {
//   let bytes = x.to_bytes(Endianness::Big);
//   let bit_slice =
//     BitSlice::from_slice(if !signed && bytes[0] == 0 { &bytes[1..] } else { &bytes });
//   let mut result = BitVec::from_bitslice(bit_slice);
//   result.force_align();
//   result
// }

pub trait BitVecConvertable {
  fn toBitVec(&self, width: usize, signed: bool) -> BitVec;
}

pub trait BigIntConvertable {
  fn toBigInt(&self, signed: bool) -> BigInt;
}

impl BitVecConvertable for BigInt {
  fn toBitVec(&self, width: usize, signed: bool) -> BitVec {
    let bytes = self.to_bytes(Endianness::Little);
    let slice = BitSlice::from_slice(&bytes);
    if slice.len() > width {
      BitVec::from_bitslice(&slice[..width])
    } else {
      let mut result = BitVec::from_bitslice(slice);
      let n = slice.len();
      if signed {
        for _ in 0..width - n {
          result.push(slice[n - 1]);
        }
      } else {
        for _ in 0..width - n {
          result.push(false);
        }
      }
      result.shrink_to_fit();
      result
    }
  }
}

impl BigIntConvertable for BitVec {
  fn toBigInt(&self, signed: bool) -> BigInt {
    let mut v = self.clone();
    let n = self.len().div_ceil(8) * 8;

    for _ in self.len()..n {
      if signed {
        v.push(self[self.len() - 1]);
      } else {
        v.push(false);
      }
    }
    if !signed && v[v.len() - 1] {
      v.push(false);
    }

    BigInt::from_bytes(v.as_raw_slice(), Endianness::Little)
  }
}

// #[test]
// fn test() -> Result<(), ()> {
//   let x = BigInt::from(-512);
//   println!("{:?}", x);
//   let y = x.toBitVec(10, true);
//   println!("{:?}", y);
//   let z = y.toBigInt(true);
//   println!("{:?}", z);

//   Ok(())
// }
