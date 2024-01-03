use arrayvec::ArrayVec;

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
