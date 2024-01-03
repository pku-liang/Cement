use irony_cmt::{DataTypeEnum, Entity, EntityId, Environ};
use itertools::multiunzip;

use super::{
  DataTypeTrait, Direction, Flip, IfcFields, Interface, SignalTrait, ConnectExpr,
};
use crate::compiler::{Cmtc, CmtcBasics};

pub trait InterfaceImpl:
  Connect<Self::FlipT> + ConnectExpr<<Self::FlipT as InterfaceImpl>::IfcT> + Sized
{
  type FlipT: InterfaceImpl<FlipT = Self>;
  type IfcT: Interface<ImplT = Self>;

  fn flip(self) -> Self::FlipT;
  fn traverse(&self) -> IfcImplFields;
  fn replace_with_fields(self, fields: IfcImplFields) -> Self;
  fn ifc(&self) -> Self::IfcT;
  #[track_caller]
  fn instance(self, c: &mut Cmtc, prefix: String) -> Self::FlipT {
    let old_fields = self.traverse();

    let FBFields {
      fwd: ImplFields { v_entity_id: arg_entities, .. },
      bwd: ImplFields { v_entity_id: output_entities, .. },
    } = old_fields.to_owned().split();

    let target_module_op_id = arg_entities
      .into_iter()
      .chain(output_entities.into_iter())
      .map(|entity_id| {
        let region_id = c
          .ir
          .get_entity(entity_id.unwrap())
          .get_parent()
          .expect("entity of ports must be in the module's body region");
        c.ir
          .get_region_use(region_id)
          .expect("The module body region must be used by the module-def op")
      })
      .reduce(
        |x, y| {
          if x != y {
            panic!("all ports must be in the same module")
          } else {
            x
          }
        },
      )
      .expect("there must be at least one port");

    let legal_prefix = c.symbol_table.get_legal_name_in_region(&c.ir, &prefix);
    let flipped_fields =
      old_fields.to_owned().ifc().flip().with_prefix(legal_prefix.to_owned()).to(c, None);

    let FBFields {
      fwd: ImplFields { v_entity_id: inputs, .. },
      bwd: ImplFields { v_entity_id: outputs, .. },
    } = flipped_fields.to_owned().split();

    // The directions here are ambiguious
    c.add_instance(target_module_op_id, legal_prefix, outputs, inputs);

    self.flip().replace_with_fields(flipped_fields)
  }
}

#[derive(Debug, Clone)]
pub struct I<T: DataTypeTrait> {
  pub signal: T,
  pub v_ir_entity_id: Vec<Option<EntityId>>,
  pub v_name: Vec<String>,
}

impl<T: DataTypeTrait> I<T> {
  pub fn new(signal: T, v_ir_entity_id: Vec<Option<EntityId>>, v_name: Vec<String>) -> Self {
    Self { signal, v_ir_entity_id, v_name }
  }
}

impl<T: DataTypeTrait> I<T> {
  pub fn data_type(&self) -> T { self.signal }
}

// InterfaceImpl<IfcT = Self, FlipT = <Self::FlipT as Interface>::ImplT>;

impl<T: DataTypeTrait+Interface<ImplT = I<T>, FlipT = Flip<T>>> InterfaceImpl for I<T> {
  type FlipT = O<T>;
  type IfcT = T;

  fn flip(self) -> Self::FlipT {
    Self::FlipT::new(self.signal, self.v_ir_entity_id, self.v_name)
  }

  fn traverse(&self) -> IfcImplFields {
    IfcImplFields::Leaf((
      self.signal.v_ir_type(),
      Direction::In,
      self.v_ir_entity_id.to_owned(),
      self.v_name.to_owned(),
    ))
  }

  fn replace_with_fields(self, fields: IfcImplFields) -> Self {
    match fields {
      IfcImplFields::Leaf((data_type, Direction::In, entity_id, name)) => {
        assert_eq!(self.signal.v_ir_type(), data_type);
        Self::new(self.signal, entity_id, name)
      },
      _ => {
        panic!(
          "DataType should be impled as Signal with a Leaf field at the In direction"
        )
      },
    }
  }

  fn ifc(&self) -> Self::IfcT { self.signal.to_owned() }
}

pub struct O<T: SignalTrait> {
  pub signal: T,
  pub v_ir_entity_id: Vec<Option<EntityId>>,
  pub v_name: Vec<String>,
}

impl<T: SignalTrait> O<T> {
  pub fn new(signal: T, v_ir_entity_id: Vec<Option<EntityId>>, v_name: Vec<String>) -> Self {
    Self { signal, v_ir_entity_id, v_name }
  }
}

impl<T: DataTypeTrait> O<T> {
  pub fn data_type(&self) -> T { self.signal }
}

impl<T: DataTypeTrait+Interface<ImplT = I<T>, FlipT = Flip<T>>> InterfaceImpl for O<T> {
  type FlipT = I<T>;
  type IfcT = <T as Interface>::FlipT;

  fn flip(self) -> Self::FlipT {
    Self::FlipT::new(self.signal, self.v_ir_entity_id, self.v_name)
  }

  fn traverse(&self) -> IfcImplFields {
    IfcImplFields::Leaf((
      self.signal.v_ir_type(),
      Direction::Out,
      self.v_ir_entity_id.to_owned(),
      self.v_name.to_owned(),
    ))
  }

  fn replace_with_fields(self, fields: IfcImplFields) -> Self {
    match fields {
      IfcImplFields::Leaf((data_type, Direction::Out, entity_id, name)) => {
        assert_eq!(self.signal.v_ir_type(), data_type);
        Self::new(self.signal, entity_id, name)
      },
      _ => {
        panic!(
          "DataType should be impled as Signal with a Leaf field at the In direction"
        )
      },
    }
  }

  fn ifc(&self) -> Self::IfcT { Flip(self.signal.to_owned()) }
}

// TODO: This should be implemented for all InterfaceImpl types
pub trait HasEntities {
  fn v_ir_entity_id(&self) -> Vec<Option<EntityId>>;
}

impl<T: DataTypeTrait> HasEntities for I<T> {
  fn v_ir_entity_id(&self) -> Vec<Option<EntityId>> { self.v_ir_entity_id.to_owned() }
}

impl<T: DataTypeTrait> HasEntities for O<T> {
  fn v_ir_entity_id(&self) -> Vec<Option<EntityId>> { self.v_ir_entity_id.to_owned() }
}

pub trait Connect<T>  {
  fn connect(self, target: T, c: &mut Cmtc);
}

impl<T: DataTypeTrait> Connect<O<T>> for I<T> {
  fn connect(self, target: O<T>, c: &mut Cmtc) { target.connect(self, c); }
}

impl<T: DataTypeTrait> Connect<I<T>> for O<T> {
  fn connect(self, target: I<T>, c: &mut Cmtc) {
    assert!(
      self.signal.total_width() == target.signal.total_width(),
      "only data type of the same width can be connected"
    );
    c.assign(self.v_ir_entity_id, target.v_ir_entity_id);
  }
}

pub type FBNames = Vec<Option<String>>;

pub fn default_names_from_ifc_fields(ifc_fields: IfcFields) -> FBNames {
  let (fwd_len, bwd_len) = ifc_fields.count();
  vec![None; fwd_len + bwd_len]
}

#[derive(Default, Debug, Clone)]
pub struct FBFields {
  pub fwd: ImplFields,
  pub bwd: ImplFields,
}

impl FBFields {
  pub fn new(
    fwd: Vec<(String, DataTypeEnum, Option<EntityId>)>,
    bwd: Vec<(String, DataTypeEnum, Option<EntityId>)>,
  ) -> Self {
    Self {
      fwd: ImplFields::from_tuple(fwd),
      bwd: ImplFields::from_tuple(bwd),
    }
  }
}

#[derive(Default, Debug, Clone)]
pub struct ImplFields {
  pub v_name: Vec<String>,
  pub v_data_type: Vec<irony_cmt::DataTypeEnum>,
  pub v_entity_id: Vec<Option<EntityId>>,
}

impl ImplFields {
  pub fn new(
    v_name: Vec<String>, v_data_type: Vec<irony_cmt::DataTypeEnum>,
    v_entity_id: Vec<Option<EntityId>>,
  ) -> Self {
    assert_eq!(v_name.len(), v_data_type.len());
    assert_eq!(v_name.len(), v_entity_id.len());
    Self { v_name, v_data_type, v_entity_id }
  }

  pub fn from_tuple(v_tuple: Vec<(String, DataTypeEnum, Option<EntityId>)>) -> Self {
    let (v_name, v_data_type, v_entity_id) = multiunzip(v_tuple);
    Self { v_name, v_data_type, v_entity_id }
  }

  pub fn append(&mut self, that: &mut ImplFields) {
    self.v_name.append(&mut that.v_name);
    self.v_data_type.append(&mut that.v_data_type);
    self.v_entity_id.append(&mut that.v_entity_id);
  }

  pub fn iter(&self) -> impl Iterator<Item = ((&String, &DataTypeEnum), &Option<EntityId>)> {
    self.v_name.iter().zip(self.v_data_type.iter()).zip(self.v_entity_id.iter())
  }

  pub fn into_iter(self) -> impl Iterator<Item = ((String, DataTypeEnum), Option<EntityId>)> {
    self
      .v_name
      .into_iter()
      .zip(self.v_data_type.into_iter())
      .zip(self.v_entity_id.into_iter())
  }

  pub fn len(&self) -> usize {
    assert_eq!(self.v_name.len(), self.v_data_type.len());
    assert_eq!(self.v_name.len(), self.v_entity_id.len());
    self.v_name.len()
  }

  pub fn zip3(self) -> Vec<(String, DataTypeEnum, Option<EntityId>)> {
    self
      .v_name
      .into_iter()
      .zip(self.v_data_type.into_iter())
      .zip(self.v_entity_id.into_iter())
      .map(|((name, data_type), entity_id)| (name, data_type, entity_id))
      .collect()
  }
}

#[derive(Debug, Clone)]
pub enum IfcImplFields {
  Leaf((Vec<irony_cmt::DataTypeEnum>, Direction, Vec<Option<EntityId>>, Vec<String>)),
  Branch(Vec<(String, IfcImplFields)>), // String here is used for ifcing to Interface
}

impl Iterator for IfcImplFields {
  type Item = IfcImplFields;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      IfcImplFields::Leaf(_) => None,
      IfcImplFields::Branch(v) => v.pop().map(|(_, val)| val),
    }
  }
}

impl IfcImplFields {
  pub fn flip(self) -> Self {
    match self {
      Self::Leaf((data_type, direction, entity_id, name)) => {
        Self::Leaf((data_type, direction.flip(), entity_id, name))
      },
      Self::Branch(v) => {
        Self::Branch(v.into_iter().map(|(name, val)| (name, val.flip())).collect())
      },
    }
  }

  pub fn ifc(self) -> IfcFields {
    match self {
      IfcImplFields::Leaf((data_type, direction, _entity_id, _name)) => {
        IfcFields::Leaf((data_type, direction))
      },
      IfcImplFields::Branch(fields) => IfcFields::Branch(
        fields.into_iter().map(|(name, val)| (name, val.ifc())).collect(),
      ),
    }
  }

  pub fn split(self) -> FBFields {
    match self {
      IfcImplFields::Leaf((data_type, direction, entity_id, names)) => match direction {
        Direction::Out => FBFields {
          fwd: ImplFields::default(),
          bwd: ImplFields::new(names, data_type, entity_id),
        },
        Direction::In => FBFields {
          fwd: ImplFields::new(names, data_type, entity_id),
          bwd: ImplFields::default(),
        },
      },
      IfcImplFields::Branch(fields) => {
        let mut args = ImplFields::default();
        let mut outputs = ImplFields::default();
        fields.into_iter().for_each(|(_, val)| {
          let FBFields { fwd: mut new_args, bwd: mut new_outputs } = val.split();
          args.append(&mut new_args);
          outputs.append(&mut new_outputs);
        });
        FBFields { fwd: args, bwd: outputs }
      },
    }
  }
}
