use std::panic::Location;

use irony::{utils, OpId, AsBool};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ClkType;

impl std::fmt::Display for ClkType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "i1") }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct UIntType(pub usize);

impl std::fmt::Display for UIntType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "i{}", self.0)
  }
}

impl Into<UIntType> for usize {
  fn into(self) -> UIntType { UIntType(self) }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct StructType(pub Vec<(String, Box<DataTypeEnum>)>);

// TODO: fix this
impl std::fmt::Display for StructType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "!hw.struct<{}>",
      self
        .0
        .iter()
        .map(|(field, ty)| format!("{}: {}", field, ty))
        .collect::<Vec<_>>()
        .join(", ")
    )
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ArrayType(pub Box<DataTypeEnum>, pub usize);

// TODO: fix this
impl std::fmt::Display for ArrayType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "!hw.array<{}x{}>", self.1, self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct UArrayType(pub Box<DataTypeEnum>, pub usize);

// TODO: fix this
impl std::fmt::Display for UArrayType {
  fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { todo!() }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct SeqHlmemType(pub Box<DataTypeEnum>, pub Vec<usize>);

// TODO: fix this
impl std::fmt::Display for SeqHlmemType {
  fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { todo!() }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum CombVariadicPredicate {
  Add,
  Mul,
  And,
  Or,
  Xor,
}

impl CombVariadicPredicate {
  pub fn get_str(&self) -> &'static str {
    match self {
      CombVariadicPredicate::Add => "add",
      CombVariadicPredicate::Mul => "mul",
      CombVariadicPredicate::And => "and",
      CombVariadicPredicate::Or => "or",
      CombVariadicPredicate::Xor => "xor",
    }
  }
}

impl std::fmt::Display for CombVariadicPredicate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.get_str())
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum CombUnaryPredicate {
  Not,
  Neg,
}

impl CombUnaryPredicate {
  pub fn get_str(&self) -> &'static str {
    match self {
      CombUnaryPredicate::Not => "not",
      CombUnaryPredicate::Neg => "neg",
    }
  }
}

impl std::fmt::Display for CombUnaryPredicate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.get_str())
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum CombBinaryPredicate {
  DivU,
  DivS,
  ModU,
  ModS,
  Shl,
  ShrU,
  ShrS,
  Sub,
}

impl CombBinaryPredicate {
  pub fn get_str(&self) -> &'static str {
    match self {
      CombBinaryPredicate::DivU => "divu",
      CombBinaryPredicate::DivS => "divs",
      CombBinaryPredicate::ModU => "modu",
      CombBinaryPredicate::ModS => "mods",
      CombBinaryPredicate::Shl => "shl",
      CombBinaryPredicate::ShrU => "shru",
      CombBinaryPredicate::ShrS => "shrs",
      CombBinaryPredicate::Sub => "sub",
    }
  }
}

impl std::fmt::Display for CombBinaryPredicate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.get_str())
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum CombICmpPredicate {
  EQ,
  NE,
  SLT,
  SLE,
  SGT,
  SGE,
  ULT,
  ULE,
  UGT,
  UGE,
  CEQ,
  CNE,
  WEQ,
  WNE,
}

impl CombICmpPredicate {
  pub fn get_str(&self) -> &'static str {
    match self {
      CombICmpPredicate::EQ => "eq",
      CombICmpPredicate::NE => "ne",
      CombICmpPredicate::SLT => "slt",
      CombICmpPredicate::SLE => "sle",
      CombICmpPredicate::SGT => "sgt",
      CombICmpPredicate::SGE => "sge",
      CombICmpPredicate::ULT => "ult",
      CombICmpPredicate::ULE => "ule",
      CombICmpPredicate::UGT => "ugt",
      CombICmpPredicate::UGE => "uge",
      CombICmpPredicate::CEQ => "ceq",
      CombICmpPredicate::CNE => "cne",
      CombICmpPredicate::WEQ => "weq",
      CombICmpPredicate::WNE => "wne",
    }
  }
}

impl std::fmt::Display for CombICmpPredicate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.get_str())
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct StringAttr(pub String);

impl Into<StringAttr> for &str {
  fn into(self) -> StringAttr { StringAttr(self.to_string()) }
}

impl Into<StringAttr> for String {
  fn into(self) -> StringAttr { StringAttr(self) }
}

impl std::fmt::Display for StringAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct BoolAttr(pub bool);

impl Into<BoolAttr> for bool {
  fn into(self) -> BoolAttr { BoolAttr(self) }
}

impl std::fmt::Display for BoolAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", if self.0 { "1" } else { "0" })
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct IdAttr(pub usize);

impl Into<IdAttr> for u32 {
  fn into(self) -> IdAttr { IdAttr(self as usize) }
}
impl Into<IdAttr> for usize {
  fn into(self) -> IdAttr { IdAttr(self) }
}

impl std::fmt::Display for IdAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct OpIdAttr(pub OpId);

impl Into<OpIdAttr> for OpId {
  fn into(self) -> OpIdAttr { OpIdAttr(self) }
}
impl Into<OpIdAttr> for usize {
  fn into(self) -> OpIdAttr { OpId(self).into() }
}

impl Into<OpId> for OpIdAttr {
  fn into(self) -> OpId { self.0 }
}

impl std::fmt::Display for OpIdAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:#?}", self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct UIntAttr(pub u32);

impl Into<UIntAttr> for u32 {
  fn into(self) -> UIntAttr { UIntAttr(self) }
}
impl Into<UIntAttr> for usize {
  fn into(self) -> UIntAttr { UIntAttr(self as u32) }
}

impl Into<u32> for UIntAttr {
  fn into(self) -> u32 { self.0 }
}

impl std::fmt::Display for UIntAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:#?}", self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct TypeAttr(pub DataTypeEnum);

impl std::fmt::Display for TypeAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ConstantAttr(pub Vec<bool>);
impl std::fmt::Display for ConstantAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", utils::arith::from_bits_to_u32(self.0.to_owned()))
  }
}
impl<const N: usize> Into<ConstantAttr> for [u32; N] {
  fn into(self) -> ConstantAttr {
    let mut v = Vec::new();
    for i in 0..N {
      v.push(self[i] != 0);
    }
    ConstantAttr(v)
  }
}

impl Into<ConstantAttr> for u32 {
  fn into(self) -> ConstantAttr { ConstantAttr(utils::arith::from_u32_to_bits(self)) }
}
impl Into<ConstantAttr> for usize {
  fn into(self) -> ConstantAttr {
    ConstantAttr(utils::arith::from_u32_to_bits(self as u32))
  }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ArrayAttr(pub Vec<AttributeEnum>);
impl std::fmt::Display for ArrayAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut sub_str = Vec::new();
    for b in &self.0 {
      sub_str.push(format!("{}", b));
    }
    write!(f, "[{}]", sub_str.join(","))
  }
}

impl AttributeEnum {
  pub fn print_for_aggregate_constant(&self, dtype: DataTypeEnum) -> String {
    match dtype {
      DataTypeEnum::UInt(uint) => {
        let AttributeEnum::ConstantAttr(constant) = self else {
          panic!("no constant attr for uint")
        };
        format!("{} : {}", constant, uint)
      },
      DataTypeEnum::Array(ArrayType(boxed, size)) => {
        let AttributeEnum::ArrayAttr(ArrayAttr(array)) = self else {
          panic!("no array attr for array")
        };
        assert!(array.len() == size);

        let sub_strs = array
          .iter()
          .map(|x| x.print_for_aggregate_constant(*boxed.to_owned()))
          .collect::<Vec<_>>();
        format!("[{}]", sub_strs.join(", "))
      },

      DataTypeEnum::Struct(StructType(v_field_type)) => {
        let AttributeEnum::ArrayAttr(ArrayAttr(array)) = self else {
          panic!("no array attr for struct")
        };

        let sub_strs = v_field_type
          .iter()
          .zip(array.iter())
          .map(|((_, dtype), attr)| attr.print_for_aggregate_constant(*dtype.to_owned()))
          .collect::<Vec<_>>();

        format!("[{}]", sub_strs.join(", "))
      },

      _ => unimplemented!(),
    }
  }
}

impl<I: Into<AttributeEnum>> Into<ArrayAttr> for Vec<I> {
  fn into(self) -> ArrayAttr { ArrayAttr(self.into_iter().map(|x| x.into()).collect()) }
}

impl Into<ArrayAttr> for () {
  fn into(self) -> ArrayAttr { ArrayAttr(Vec::<AttributeEnum>::new()) }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct LocationAttr(pub Location<'static>);

impl From<&Location<'static>> for LocationAttr {
  fn from(location: &Location<'static>) -> Self { LocationAttr(location.to_owned()) }
}

impl std::fmt::Display for LocationAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

irony::data_type_enum![
    DataTypeEnum = {
        Clk(ClkType),
        UInt(UIntType),
        Struct(StructType),
        Array(ArrayType),
        UArray(UArrayType),
        SeqHlmem(SeqHlmemType),
    }
];

irony::attribute_enum! {
    [data_type = DataTypeEnum]
    AttributeEnum = {
        ConstantAttr(ConstantAttr),
        BoolAttr(BoolAttr),
        LocationAttr(LocationAttr),
        IdAttr(IdAttr),
        UIntAttr(UIntAttr),
        OpIdAttr(OpIdAttr),
        StringAttr(StringAttr),
        TypeAttr(TypeAttr),
        ArrayAttr(ArrayAttr),
        CombVariadicPredicate(CombVariadicPredicate),
        CombBinaryPredicate(CombBinaryPredicate),
        CombUnaryPredicate(CombUnaryPredicate),
        CombICmpPredicate(CombICmpPredicate)
    }
}

impl DataTypeEnum {
  pub fn width(&self) -> usize {
    match self {
      DataTypeEnum::UInt(UIntType(width)) => *width,
      DataTypeEnum::Array(ArrayType(boxed, size)) => boxed.width() * size,
      DataTypeEnum::Struct(StructType(v_field_type)) => {
        v_field_type.iter().map(|(_, dtype)| dtype.width()).sum()
      },
      DataTypeEnum::Clk(_) => 1,
      _ => unimplemented!(),
    }
  }
}

impl AsBool for AttributeEnum {
  fn as_bool(&self) -> bool {
    match self {
      AttributeEnum::BoolAttr(BoolAttr(b)) => *b,
      _ => panic!("not a bool attr"),
    }
  }
}