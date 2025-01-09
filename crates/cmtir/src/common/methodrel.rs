use super::*;

#[derive(Debug, Clone, Copy, PartialEq, SExpr)]
pub enum MethodRel {
  #[pp(lowercase = false)]
  CF,
  C,
  SA,
  SB,
}

impl MethodRel {
  pub fn to_string(&self) -> String {
    self.ir_dump()
  }
  pub fn from_str(s: &str) -> Result<MethodRel, String> {
    match s {
      "CF" => Ok(MethodRel::CF),
      "C" => Ok(MethodRel::C),
      "SA" => Ok(MethodRel::SA),
      "SB" => Ok(MethodRel::SB),
      _ => Err(format!("Invalid method relation: {}", s)),
    }
  }

  pub fn join(self, other: Self) -> Self {
    match (self, other) {
      (MethodRel::CF, x) => x,
      (x, MethodRel::CF) => x,
      (MethodRel::C, _) => MethodRel::C,
      (_, MethodRel::C) => MethodRel::C,
      (MethodRel::SA, MethodRel::SA) => MethodRel::SA,
      (MethodRel::SB, MethodRel::SB) => MethodRel::SB,
      _ => MethodRel::CF,
    }
  }

  pub fn rev(self) -> Self {
    match self {
      MethodRel::CF => MethodRel::CF,
      MethodRel::C => MethodRel::C,
      MethodRel::SA => MethodRel::SB,
      MethodRel::SB => MethodRel::SA,
    }
  }
}

impl ToString for MethodRel {
  fn to_string(&self) -> String {
    match self {
      MethodRel::CF => "CF",
      MethodRel::C => "C",
      MethodRel::SA => "SA",
      MethodRel::SB => "SB",
    }
    .to_string()
  }
}
