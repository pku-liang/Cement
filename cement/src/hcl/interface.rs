use std::fmt::Debug;

use irony_cmt::{DataTypeEnum, EntityId};

use super::{FBFields, IfcImplFields, InterfaceImpl, I, DataTypeTrait};
use crate::compiler::{Cmtc, CmtcBasics};

pub trait Interface: Clone + Debug {
  type FlipT: Interface<FlipT = Self>;
  type ImplT: InterfaceImpl<IfcT = Self, FlipT = <Self::FlipT as Interface>::ImplT>
    + 'static;
  fn name() -> String;
  fn flip(self) -> Self::FlipT;
  fn traverse(&self) -> IfcFields;
  fn impl_with(self, fields: IfcImplFields) -> Self::ImplT;
}


impl<T: DataTypeTrait> Interface for T {
  type FlipT = Flip<Self>;
  type ImplT = I<Self>;

  fn name() -> String { "DataType".to_string() }

  fn flip(self) -> Self::FlipT { Flip(self) }

  fn traverse(&self) -> IfcFields { IfcFields::Leaf((self.v_ir_type(), Direction::In)) }

  fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
    // println!("signal impl with {:?}", fields);
    match fields {
      IfcImplFields::Leaf((data_type, Direction::In, entity_id, name)) => {
        assert_eq!(self.v_ir_type(), data_type);
        I::new(self, entity_id, name)
      },
      _ => {
        panic!("DataType should be implemented as Signal with a Leaf field at the In direction")
      },
    }
  }
}


#[derive(Clone, Debug, Copy, Default)]
pub struct Flip<T: DataTypeTrait>(pub T);


impl<T: DataTypeTrait> Interface for Flip<T>
{
  type FlipT = T;
  type ImplT = <T::ImplT as InterfaceImpl>::FlipT;

  fn name() -> String { T::name() + "_flip" }

  fn flip(self) -> Self::FlipT { self.0 }

  fn traverse(&self) -> IfcFields { self.0.traverse().flip() }

  fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
    let inner = self.0.impl_with(fields.flip());
    inner.flip()
  }
}

#[derive(Debug, Clone)]
pub enum Direction {
  In,
  Out,
}
impl Direction {
  pub fn flip(self) -> Self {
    match self {
      Direction::In => Direction::Out,
      Direction::Out => Direction::In,
    }
  }
}

#[derive(Debug, Clone)]
pub enum IfcFields {
  Leaf((Vec<irony_cmt::DataTypeEnum>, Direction)),
  Branch(Vec<(String, IfcFields)>),
}

pub type IfcImplField = ((String, DataTypeEnum), Option<EntityId>);

impl IfcFields {
  pub fn with_prefix(self, prefix: String) -> Self {
    match self {
      Self::Branch(v) => Self::Branch(
        v.into_iter().map(|(name, val)| (format!("{}.{}", prefix, name), val)).collect(),
      ),
      _ => {
        panic!("with_prefix only be applied to Branch")
      },
    }
  }

  pub fn flip(self) -> Self {
    match self {
      Self::Leaf((data_type, direction)) => Self::Leaf((data_type, direction.flip())),
      Self::Branch(v) => {
        Self::Branch(v.into_iter().map(|(name, val)| (name, val.flip())).collect())
      },
    }
  }

  pub fn count(&self) -> (usize, usize) {
    match self {
      Self::Leaf((v_data_type, direction)) => match direction {
        Direction::In => (v_data_type.len(), 0),
        Direction::Out => (0, v_data_type.len()),
      },
      Self::Branch(v_field) => {
        let mut count = (0, 0);
        for (_, field) in v_field {
          let (in_count, out_count) = field.count();
          count.0 += in_count;
          count.1 += out_count;
        }
        count
      },
    }
  }

  pub fn with_fb_fields(self, fb_impl_fields: FBFields) -> IfcImplFields {
    self.with_fb_fields_iter(
      &mut fb_impl_fields.fwd.into_iter(),
      &mut fb_impl_fields.bwd.into_iter(),
    )
  }

  pub fn with_fb_fields_iter(
    self, fwd_iter: &mut impl Iterator<Item = IfcImplField>,
    bwd_iter: &mut impl Iterator<Item = IfcImplField>,
  ) -> IfcImplFields {
    match self {
      Self::Leaf((v_data_type, direction)) => match direction {
        Direction::In => {
          let mut v_entity_id = Vec::new();
          let mut v_name = Vec::new();
          for _ in v_data_type.iter() {
            let fwd_item = fwd_iter.next().expect("not enough entity_id");
            v_entity_id.push(fwd_item.1);
            v_name.push(fwd_item.0 .0);
          }
          IfcImplFields::Leaf((v_data_type, direction, v_entity_id, v_name))
        },
        Direction::Out => {
          let mut v_entity_id = Vec::new();
          let mut v_name = Vec::new();
          for _ in v_data_type.iter() {
            let bwd_item = bwd_iter.next().expect("not enough entity_id");
            v_entity_id.push(bwd_item.1);
            v_name.push(bwd_item.0 .0);
          }
          IfcImplFields::Leaf((v_data_type, direction, v_entity_id, v_name))
        },
      },
      Self::Branch(v_field) => IfcImplFields::Branch(
        v_field
          .into_iter()
          .map(|(name, val)| (name, val.with_fb_fields_iter(fwd_iter, bwd_iter)))
          .collect(),
      ),
    }
  }

  pub fn names(&self, prefix: Option<String>) -> Vec<Option<String>> {
    match self {
      Self::Leaf((data_type, _)) => {
        if data_type.len() > 1 {
          (0..data_type.len())
            .map(|i| match prefix.to_owned() {
              Some(prefix) => Some(format!("{}.{}", prefix, i)),
              None => Some(i.to_string()),
            })
            .collect()
        } else {
          vec![prefix.clone()]
        }
      },
      Self::Branch(fields) => fields
        .iter()
        .map(|(name, val)| {
          val.names(match prefix.to_owned() {
            None => Some(name.to_owned()),
            Some(prefix) => Some(format!("{}.{}", prefix, name)),
          })
        })
        .flatten()
        .collect(),
    }
  }

  #[track_caller]
  pub fn to(self, c: &mut Cmtc, prefix: Option<String>) -> IfcImplFields {
    match self {
      // TODO: [HCL-safety] the prefix should be the final name of the wire
      Self::Leaf((data_type, direction)) => {
        let (entity, name): (Vec<_>, _) = c
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

        IfcImplFields::Leaf((data_type.to_owned(), direction, entity.into_iter().map(|x| Some(x)).collect(), name))
      },
      Self::Branch(fields) => {
        let mut v = Vec::new();
        for (name, val) in fields {
          v.push((
            name.to_owned(),
            val.to(c, match prefix.to_owned() {
              None => Some(name),
              Some(prefix) => Some(format!("{}.{}", prefix, name)),
            }),
          ));
        }

        IfcImplFields::Branch(v)
      },
    }
  }
}
