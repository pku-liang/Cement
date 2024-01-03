pub fn extract_vec<A: Clone + PartialEq>(
  v: &Vec<(String, A)>, field_name: &str,
) -> Option<A> {
  v.iter().find_map(
    |(name, field)| {
      if name == field_name {
        Some(field.to_owned())
      } else {
        None
      }
    },
  )
}

pub mod print {
  pub fn tab(lines: String) -> String {
    lines.lines().map(|line| format!("\t{}\n", line)).collect()
  }
  pub fn from_bits_to_str(bits: Vec<bool>) -> String {
    bits.iter().map(|bit| if *bit { "1" } else { "0" }).collect()
  }
}

pub mod arith {
  pub fn from_bits_to_u32(bits: Vec<bool>) -> u32 {
    let mut sum: u32 = 0;
    for (i, bit) in bits.iter().enumerate() {
      sum += (*bit as u32) << (i as u32);
    }
    sum
  }

  pub fn from_u32_to_bits(val: u32) -> Vec<bool> {
    let mut bits: Vec<bool> = Vec::new();
    let mut val = val;
    while val > 0 {
      bits.push(val % 2 == 1);
      val /= 2;
    }
    bits
  }
}
