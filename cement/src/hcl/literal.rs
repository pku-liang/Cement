use std::ops::Deref;

use super::*;
use crate::compiler::{Cmtc, CmtcBasics};
use crate::utils;

#[derive(Debug, Clone)]
pub struct BitsValue {
  pub data: Vec<bool>,
}

impl Deref for BitsValue {
  type Target = Vec<bool>;

  fn deref(&self) -> &Self::Target { self.data.as_ref() }
}

#[derive(Debug, Clone)]
pub struct NestedValue {
  pub data: Vec<DataValue>,
}

impl Deref for NestedValue {
  type Target = Vec<DataValue>;

  fn deref(&self) -> &Self::Target { self.data.as_ref() }
}

#[derive(Debug, Clone)]
pub enum DataValue {
  Bits(BitsValue),
  Nested(NestedValue),
}

impl IfcFields {
  #[track_caller]
  pub fn to_with_constant(
    self, c: &mut Cmtc, prefix: Option<String>,
    v_data: &mut impl Iterator<Item = DataValue>,
  ) -> IfcImplFields {
    match self {
      // TODO: [HCL-safety] the prefix should be the final name of the wire
      Self::Leaf((data_type, direction)) => {
        let (entity, name): (Vec<_>, Vec<_>) = c
          .add_wires(
            data_type.to_owned(),
            if data_type.len() > 1 {
              (0..data_type.len())
                .map(|i| {
                  format!("{}.{}", prefix.clone().expect("leaf field must have name"), i)
                })
                .collect()
            } else {
              vec![prefix.clone().unwrap()]
            },
          )
          .into_iter()
          .unzip();

        for (entity, data_type) in entity.iter().zip(data_type.iter()) {
          c.add_constant(entity.to_owned(), data_type.to_owned(), v_data.next().unwrap());
        }

        IfcImplFields::Leaf((data_type.to_owned(), direction, entity.into_iter().map(|x| Some(x)).collect(), name))
      },
      Self::Branch(fields) => {
        let mut v = Vec::new();
        for (name, val) in fields {
          v.push((
            name.to_owned(),
            val.to_with_constant(
              c,
              match prefix.to_owned() {
                None => Some(name),
                Some(prefix) => Some(format!("{}.{}", prefix, name)),
              },
              v_data,
            ),
          ));
        }

        IfcImplFields::Branch(v)
      },
    }
  }
}

#[derive(Debug, Clone)]
pub struct SignalValue {
  pub v_data: Vec<DataValue>,
  pub name: String,
}

pub trait IntoValue<T: SignalTrait> {
  fn into_value(self, signal: T) -> SignalValue;
}

pub trait Literal<T: SignalTrait + Interface> {
  fn lit(self, data_type: T) -> Expr<T>;
}

impl<T: SignalTrait, D: IntoValue<T>> Literal<T> for D {
  fn lit(self, signal: T) -> Expr<T> {
    let SignalValue { v_data, name } = self.into_value(signal.to_owned());

    Expr {
      ifc: signal.to_owned(),
      ast: ExprAst::Branch(
        ExprNode::Constant(Constant {
          ifc_fields: signal.to_owned().traverse(),
          v_data,
        }),
        Vec::new(),
        Some(vec![Some(name)]),
        signal.to_owned().traverse(),
      ),
    }
  }
}

impl IntoValue<B<1>> for bool {
  fn into_value(self, _: B<1>) -> SignalValue {
    SignalValue {
      v_data: vec![DataValue::Bits(BitsValue { data: vec![self] })],
      name: format!("{}", self),
    }
  }
}

impl Into<SignalValue> for bool {
  fn into(self) -> SignalValue { self.into_value(B1) }
}

impl<const N: usize> IntoValue<B<N>> for [bool; N] {
  fn into_value(self, _: B<N>) -> SignalValue {
    SignalValue {
      v_data: vec![DataValue::Bits(BitsValue { data: self.to_vec() })],
      name: format!("b{}", utils::bits_str(self.to_vec())),
    }
  }
}

impl<const N: usize> Into<SignalValue> for [bool; N] {
  fn into(self) -> SignalValue { self.into_value(B::<N>) }
}

impl<const N: usize> IntoValue<B<N>> for Vec<bool> {
  fn into_value(self, _: B<N>) -> SignalValue {
    assert!(self.len() == N);

    SignalValue {
      v_data: vec![DataValue::Bits(BitsValue { data: self.to_owned() })],
      name: format!("b{}", utils::bits_str(self)),
    }
  }
}

impl Into<SignalValue> for Vec<bool> {
  fn into(self) -> SignalValue {
    let len = self.len();
    self.into_value(Bits(len))
  }
}

macro_rules! unsigned_lit_u {
  ($type:ty) => {
    impl<const N: usize> IntoValue<B<N>> for $type {
      fn into_value(self, data_type: B<N>) -> SignalValue {
        SignalValue {
          v_data: vec![DataValue::Bits(BitsValue {
            data: utils::usize_to_bitvec(data_type.width(), self as usize),
          })],
          name: format!("{}.{}", stringify!($type), self),
        }
      }
    }
  };
}

unsigned_lit_u!(u8);
unsigned_lit_u!(u16);
unsigned_lit_u!(u32);
unsigned_lit_u!(u64);
unsigned_lit_u!(u128);
unsigned_lit_u!(usize);
unsigned_lit_u!(i8);
unsigned_lit_u!(i16);
unsigned_lit_u!(i32);
unsigned_lit_u!(i64);
unsigned_lit_u!(i128);

impl<const N: usize> IntoValue<Bits> for [bool; N] {
  fn into_value(self, data_type: Bits) -> SignalValue {
    assert!(N == data_type.width());

    SignalValue {
      v_data: vec![DataValue::Bits(BitsValue { data: self.to_vec() })],
      name: format!("b{}", utils::bits_str(self.to_vec())),
    }
  }
}

impl IntoValue<Bits> for Vec<bool> {
  fn into_value(self, data_type: Bits) -> SignalValue {
    assert!(self.len() == data_type.width());

    SignalValue {
      v_data: vec![DataValue::Bits(BitsValue { data: self.to_owned() })],
      name: format!("b{}", utils::bits_str(self)),
    }
  }
}

macro_rules! unsigned_lit_uint {
  ($type:ty) => {
    impl IntoValue<Bits> for $type {
      fn into_value(self, data_type: Bits) -> SignalValue {
        SignalValue {
          v_data: vec![DataValue::Bits(BitsValue {
            data: utils::usize_to_bitvec(data_type.width(), self as usize),
          })],
          name: format!("{}.{}", stringify!($type), self),
        }
      }
    }
  };
}

unsigned_lit_uint!(u8);
unsigned_lit_uint!(u16);
unsigned_lit_uint!(u32);
unsigned_lit_uint!(u64);
unsigned_lit_uint!(u128);
unsigned_lit_uint!(usize);
unsigned_lit_uint!(i8);
unsigned_lit_uint!(i16);
unsigned_lit_uint!(i32);
unsigned_lit_uint!(i64);
unsigned_lit_uint!(i128);

impl<const N: usize, T: DataTypeTrait, S: IntoValue<T>> IntoValue<Arr<N, T>> for [S; N] {
  fn into_value(self, data_type: Arr<N, T>) -> SignalValue {
    let (data, name) = self
      .into_iter()
      .map(|v| {
        let SignalValue { v_data, name } = v.into_value(data_type.0);
        assert_eq!(v_data.len(), 1);
        (v_data.into_iter().next().expect("must have one data"), name)
      })
      .unzip::<_, _, Vec<_>, Vec<_>>();

    SignalValue {
      v_data: vec![DataValue::Nested(NestedValue { data })],
      name: format!("arr_{}", name.first().expect("must have one value")),
    }
  }
}

impl<const N: usize, T: DataTypeTrait, S: IntoValue<T>> IntoValue<Arr<N, T>> for Vec<S> {
  fn into_value(self, data_type: Arr<N, T>) -> SignalValue {
    assert!(self.len() == N);
    let (data, name) = self
      .into_iter()
      .map(|v| {
        let SignalValue { v_data, name } = v.into_value(data_type.0);
        assert_eq!(v_data.len(), 1);
        (v_data.into_iter().next().expect("must have one data"), name)
      })
      .unzip::<_, _, Vec<_>, Vec<_>>();
    SignalValue {
      v_data: vec![DataValue::Nested(NestedValue { data })],
      name: format!("arr_{}", name.first().expect("must have one value")),
    }
  }
}

impl<const N: usize, T: DataTypeTrait, S: IntoValue<T>> IntoValue<Array<T>> for [S; N] {
  fn into_value(self, data_type: Array<T>) -> SignalValue {
    assert!(N == data_type.0);
    let (data, name) = self
      .into_iter()
      .map(|v| {
        let SignalValue { v_data, name } = v.into_value(data_type.1);
        assert_eq!(v_data.len(), 1);
        (v_data.into_iter().next().expect("must have one data"), name)
      })
      .unzip::<_, _, Vec<_>, Vec<_>>();
    SignalValue {
      v_data: vec![DataValue::Nested(NestedValue { data })],
      name: format!("array_{}", name.first().expect("must have one value")),
    }
  }
}

impl<T: DataTypeTrait, S: IntoValue<T>> IntoValue<Array<T>> for Vec<S> {
  fn into_value(self, data_type: Array<T>) -> SignalValue {
    assert!(self.len() == data_type.0);
    let (data, name) = self
      .into_iter()
      .map(|v| {
        let SignalValue { v_data, name } = v.into_value(data_type.1);
        assert_eq!(v_data.len(), 1);
        (v_data.into_iter().next().expect("must have one data"), name)
      })
      .unzip::<_, _, Vec<_>, Vec<_>>();
    SignalValue {
      v_data: vec![DataValue::Nested(NestedValue { data })],
      name: format!("array_{}", name.first().expect("must have one value")),
    }
  }
}
