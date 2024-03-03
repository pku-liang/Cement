use std::sync::{Arc, RwLock};

use bitvec::prelude as bv;
use visible::StructFields;

use crate::utils::{BigInt, BigIntConvertable, BitVec, BitVecConvertable};

#[derive(Clone, Eq, PartialEq, Copy, Debug)]
pub struct StateId(usize);

impl StateId {
  pub fn write_to(&self, data: StateData, container: &Arc<RwLock<SimStateContainer>>) {
    container.write().unwrap().write(*self, data);
  }

  pub fn read_from(&self, container: &Arc<RwLock<SimStateContainer>>) -> StateData {
    container.read().unwrap().read(*self)
  }
}

#[derive(Clone, Debug)]
pub struct SimStateContainer {
  pub data: Vec<StateData>,
}

impl SimStateContainer {
  pub fn new() -> Self { SimStateContainer { data: Vec::new() } }

  pub fn alloc(&mut self, data: StateData) -> StateId {
    self.data.push(data);
    return StateId(self.data.len() - 1);
  }

  pub fn read(&self, id: StateId) -> StateData {
    return self.data.get(id.0).unwrap().clone();
  }

  pub fn write(&mut self, id: StateId, data: StateData) {
    *self.data.get_mut(id.0).unwrap() = data;
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateData {
  Bits(BitsStateData),
  Aggregate(AggregateStateData),
}

impl StateData {
  pub fn new_bits(data: BitVec, signed: bool) -> Self {
    StateData::Bits(BitsStateData::new(data, signed))
  }

  pub fn new_usize(data: usize, width: usize) -> Self {
    StateData::new_bigint(&BigInt::from(data), width, false)
  }

  pub fn new_isize(data: isize, width: usize) -> Self {
    StateData::new_bigint(&BigInt::from(data), width, true)
  }

  pub fn new_bigint(data: &BigInt, width: usize, signed: bool) -> Self {
    StateData::new_bits(data.toBitVec(width, signed), signed)
  }

  pub fn new_bool(data: bool) -> Self {
    if data {
      StateData::new_bits(BitVec::from_bitslice(bv::bits![u8, bv::Lsb0; 1]), false)
    } else {
      StateData::new_bits(BitVec::from_bitslice(bv::bits![u8, bv::Lsb0; 0]), false)
    }
    // StateData::new_bigint(&BigInt::from(data), 1, false)
  }

  pub fn new_bool_vec(data: &[bool], signed: bool) -> Self {
    let mut v = BitVec::with_capacity(data.len());
    for x in data.iter() {
      v.push(*x);
    }
    StateData::new_bits(v, signed)
  }

  pub fn new_aggregate(children: impl Iterator<Item = StateData>) -> Self {
    StateData::Aggregate(AggregateStateData {
      children: Vec::from_iter(children.into_iter().map(|x| Box::new(x))),
    })
  }

  pub fn empty_bits(width: usize, signed: bool) -> Self {
    let mut result = BitVec::new();
    result.resize(width, false);
    StateData::new_bits(result, signed)
  }

  // pub fn write_to(&self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   let mut c = container.write().unwrap();
  //   let target = c.write(id);
  //   self.do_write_to(target);
  // }

  // pub fn read_from(&mut self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   let c = container.read().unwrap();
  //   let target = c.read(id);
  //   self.do_read_from(target);
  // }

  pub fn write_to(&self, target: &mut StateData) {
    match self {
      StateData::Bits(bits) => bits.write_to(target),
      StateData::Aggregate(agg) => agg.write_to(target),
    }
  }

  pub fn read_from(&mut self, target: &StateData) {
    match self {
      StateData::Bits(bits) => bits.read_from(target),
      StateData::Aggregate(agg) => agg.read_from(target),
    }
  }

  pub fn as_bool(&self) -> bool {
    if let StateData::Bits(bits) = self {
      assert!(bits.data.len() == 1 && bits.signed == false);
      bits.data[0]
    } else {
      panic!("Cannot convert aggregated type into bool!")
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[StructFields(pub)]
pub struct BitsStateData {
  data: BitVec,
  signed: bool,
}

impl BitsStateData {
  pub fn new(data: BitVec, signed: bool) -> Self { BitsStateData { data, signed } }

  // pub fn write_to(&self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   self.do_write_to(container.write().unwrap().write(id));
  // }

  // pub fn read_from(&mut self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   self.do_read_from(container.read().unwrap().read(id));
  // }

  pub fn write_to(&self, target: &mut StateData) {
    if let StateData::Bits(bits) = target {
      // Do not check signed
      assert!(bits.data.len() == self.data.len());

      bits.data.clone_from(&self.data);
    } else {
      panic!("Cannot write to unmatched type!")
    }
  }

  pub fn read_from(&mut self, target: &StateData) {
    if let StateData::Bits(bits) = target {
      // Do not check signed
      assert!(bits.data.len() == self.data.len());

      self.data.clone_from(&bits.data);
    } else {
      panic!("Cannot read from unmatched type!")
    }
  }
}

#[derive(Clone, Debug, Eq)]
#[StructFields(pub)]
pub struct AggregateStateData {
  children: Vec<Box<StateData>>,
}

impl AggregateStateData {
  pub fn new() -> Self { Self { children: Vec::new() } }

  // pub fn write_to(&self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   self.do_write_to(container.write().unwrap().write(id));
  // }

  // pub fn read_from(&mut self, id: StateId, container: &Arc<RwLock<SimStateContainer>>) {
  //   self.do_read_from(container.read().unwrap().read(id));
  // }

  pub fn write_to(&self, target: &mut StateData) {
    if let StateData::Aggregate(agg) = target {
      assert!(self.children.len() == agg.children.len());
      for (x, y) in self.children.iter().zip(agg.children.iter_mut()) {
        x.write_to(y);
      }
    } else {
      panic!("Cannot write to unmatched type!")
    }
  }

  pub fn read_from(&mut self, target: &StateData) {
    if let StateData::Aggregate(agg) = target {
      assert!(self.children.len() == agg.children.len());
      for (x, y) in self.children.iter_mut().zip(agg.children.iter()) {
        x.read_from(y);
      }
    } else {
      panic!("Cannot write to unmatched type!")
    }
  }
}

impl std::cmp::PartialEq for AggregateStateData {
  fn eq(&self, other: &Self) -> bool {
    self.children.iter().zip(other.children.iter()).all(|(x, y)| x == y)
  }
}
