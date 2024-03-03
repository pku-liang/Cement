use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use bitvec::prelude as bv;
use irony_cmt::{CombBinaryPredicate, CombICmpPredicate, CombVariadicPredicate};
use visible::StructFields;

use super::state::*;
use crate::utils::{BigIntConvertable, BitSlice, BitVec, BitVecConvertable};

pub trait SimEvent: Debug {
  fn run(&self);
}

pub type BoxEvent = Box<dyn SimEvent + Send + Sync>;

#[StructFields(pub)]
#[derive(Clone)]
pub struct PokeEvent {
  container: Arc<RwLock<SimStateContainer>>,
  id: StateId,
  data: StateData,
}

impl SimEvent for PokeEvent {
  fn run(&self) { self.id.write_to(self.data.clone(), &self.container); }
}

impl Debug for PokeEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PokeEvent").field("id", &self.id).field("data", &self.data).finish()
  }
}

// #[StructFields(pub)]
// #[derive(Clone, Debug)]
// pub struct PeekEvent {
//   ready: Arc<RwLock<bool>>,
// }

// impl SimEvent for PeekEvent {
//   fn run(&self) { *self.ready.write().unwrap() = true; }
// }

#[StructFields(pub)]
#[derive(Clone)]
pub struct AssignEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  rhs: StateId,
}

impl SimEvent for AssignEvent {
  fn run(&self) {
    let data = self.rhs.read_from(&self.container);
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for AssignEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("AssignEvent").field("lhs", &self.lhs).field("rhs", &self.rhs).finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CastEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  rhs: StateId,
}

impl SimEvent for CastEvent {
  fn run(&self) {
    let mut v = BitVec::new();
    let input = self.rhs.read_from(&self.container);
    self.read_bits(&input, &mut v);
    let mut output = self.lhs.read_from(&self.container);
    self.write_bits(&v, &mut output);
    self.lhs.write_to(output, &self.container)
  }
}

impl Debug for CastEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CastEvent").field("lhs", &self.lhs).field("rhs", &self.rhs).finish()
  }
}

impl CastEvent {
  fn read_bits(&self, x: &StateData, result: &mut BitVec) {
    match x {
      StateData::Bits(bits) => result.extend_from_bitslice(&bits.data),
      StateData::Aggregate(agg) => {
        for c in agg.children.iter() {
          self.read_bits(c, result);
        }
      },
    }
  }

  fn write_bits<'a: 'b, 'b>(
    &self, mut data: &'a BitSlice, y: &'b mut StateData,
  ) -> &'a BitSlice {
    match y {
      StateData::Bits(bits) => {
        let (a, b) = data.split_at(bits.data.len());
        bits.data.clone_from_bitslice(a);
        b
      },
      StateData::Aggregate(agg) => {
        for child in agg.children.iter_mut() {
          data = self.write_bits(data, child);
        }
        data
      },
    }
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct MuxAssignEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  sel: StateId,
  rhs0: StateId,
  rhs1: StateId,
}

impl SimEvent for MuxAssignEvent {
  fn run(&self) {
    let sel = self.sel.read_from(&self.container).as_bool();
    let data = if sel {
      self.rhs1.read_from(&self.container)
    } else {
      self.rhs0.read_from(&self.container)
    };
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for MuxAssignEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MuxAssignEvent")
      .field("lhs", &self.lhs)
      .field("sel", &self.sel)
      .field("rhs0", &self.rhs0)
      .field("rhs1", &self.rhs1)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CombVariadicEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  ops: Vec<StateId>,
  predicate: CombVariadicPredicate,
}

impl SimEvent for CombVariadicEvent {
  fn run(&self) {
    assert!(
      self.ops.len() > 1,
      "Can not do variadic arithmetic on less than two operands"
    );

    let lhs_data = self.lhs.read_from(&self.container);

    let (width, signed) = if let StateData::Bits(bits) = &lhs_data {
      (bits.data.len(), bits.signed)
    } else {
      panic!("Can not do arithmetic operations on aggregated types!")
    };

    let ops_data =
      Vec::from_iter(self.ops.iter().map(|x| x.read_from(&self.container)).map(|x| {
        if let StateData::Bits(y) = x {
          assert!(
            y.data.len() == width && y.signed == signed,
            "CombVariadic operands type inconsistent!"
          );
          y.data.clone()
        } else {
          panic!("Can no do arithmetic operations on aggregated types")
        }
      }));

    let result = match self.predicate {
      CombVariadicPredicate::Add => StateData::new_bigint(
        &ops_data.into_iter().map(|x| x.toBigInt(signed)).reduce(|x, y| x + y).unwrap(),
        width,
        signed,
      ),
      CombVariadicPredicate::Mul => StateData::new_bigint(
        &ops_data.into_iter().map(|x| x.toBigInt(signed)).reduce(|x, y| x * y).unwrap(),
        width,
        signed,
      ),
      CombVariadicPredicate::And => {
        StateData::new_bits(ops_data.into_iter().reduce(|x, y| x & y).unwrap(), signed)
      },
      CombVariadicPredicate::Or => {
        StateData::new_bits(ops_data.into_iter().reduce(|x, y| x | y).unwrap(), signed)
      },
      CombVariadicPredicate::Xor => {
        StateData::new_bits(ops_data.into_iter().reduce(|x, y| x ^ y).unwrap(), signed)
      },
    };
    self.lhs.write_to(result, &self.container);
  }
}

impl Debug for CombVariadicEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CombVariadicEvent")
      .field("lhs", &self.lhs)
      .field("ops", &self.ops)
      .field("pred", &self.predicate)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CombBinaryEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  op0: StateId,
  op1: StateId,
  predicate: CombBinaryPredicate,
}

impl SimEvent for CombBinaryEvent {
  fn run(&self) {
    let StateData::Bits(lhs) = self.lhs.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };
    let StateData::Bits(op0) = self.op0.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };
    let StateData::Bits(op1) = self.op1.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };

    assert!(
      lhs.data.len() == op0.data.len()
        && op0.data.len() == op1.data.len()
        && lhs.signed == op0.signed
        && op0.signed == op1.signed,
      "CombBinary type inconsistent!",
    );

    let op0 = op0.data.toBigInt(op0.signed);
    let op1 = op1.data.toBigInt(op1.signed);
    // TODO: find out difference between U/S
    let result = StateData::new_bigint(
      &match self.predicate {
        CombBinaryPredicate::DivU => op0 / op1,
        CombBinaryPredicate::DivS => op0 / op1,
        CombBinaryPredicate::ModU => op0 % op1,
        CombBinaryPredicate::ModS => op0 % op1,
        CombBinaryPredicate::Shl => op0 << op1,
        CombBinaryPredicate::ShrU => op0 >> op1,
        CombBinaryPredicate::ShrS => op0 >> op1,
        CombBinaryPredicate::Sub => op0 - op1,
      },
      lhs.data.len(),
      lhs.signed,
    );

    self.lhs.write_to(result, &self.container);
  }
}

impl Debug for CombBinaryEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CombBinaryEvent")
      .field("lhs", &self.lhs)
      .field("op0", &self.op0)
      .field("op1", &self.op1)
      .field("pred", &self.predicate)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CombICmpEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  op0: StateId,
  op1: StateId,
  predicate: CombICmpPredicate,
}

impl SimEvent for CombICmpEvent {
  fn run(&self) {
    let StateData::Bits(lhs) = self.lhs.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };
    let StateData::Bits(op0) = self.op0.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };
    let StateData::Bits(op1) = self.op1.read_from(&self.container) else {
      panic!("Can not do arithmetic on aggregated types!")
    };

    assert!(
      op0.data.len() == op1.data.len() && op0.signed == op1.signed,
      "CombICmp type inconsistent!",
    );

    assert!(lhs.data.len() == 1 && !lhs.signed, "CombICmp output should be a bool!");

    let op0 = op0.data.toBigInt(op0.signed);
    let op1 = op1.data.toBigInt(op1.signed);
    // TODO: find out difference between U/S _/C/W
    let result = StateData::new_bool(match self.predicate {
      CombICmpPredicate::EQ => op0 == op1,
      CombICmpPredicate::CEQ => op0 == op1,
      CombICmpPredicate::WEQ => op0 == op1,
      CombICmpPredicate::NE => op0 != op1,
      CombICmpPredicate::CNE => op0 != op1,
      CombICmpPredicate::WNE => op0 != op1,
      CombICmpPredicate::SLT => op0 < op1,
      CombICmpPredicate::ULT => op0 < op1,
      CombICmpPredicate::SLE => op0 <= op1,
      CombICmpPredicate::ULE => op0 <= op1,
      CombICmpPredicate::SGT => op0 > op1,
      CombICmpPredicate::UGT => op0 > op1,
      CombICmpPredicate::SGE => op0 >= op1,
      CombICmpPredicate::UGE => op0 >= op1,
    });
    self.lhs.write_to(result, &self.container);
  }
}

impl Debug for CombICmpEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CombICmpEvent")
      .field("lhs", &self.lhs)
      .field("op0", &self.op0)
      .field("op1", &self.op1)
      .field("pred", &self.predicate)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CombConcatEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  ops: Vec<StateId>,
}

impl SimEvent for CombConcatEvent {
  fn run(&self) {
    let StateData::Bits(lhs) = self.lhs.read_from(&self.container) else {
      panic!("Cannot concat aggregated type!")
    };

    let mut result = BitVec::new();
    for op in &self.ops {
      let x = op.read_from(&self.container);
      if let StateData::Bits(y) = x {
        let mut y = y.data.clone();
        let slice = y.as_mut_bitslice();
        slice.reverse();
        result.extend_from_bitslice(slice);
      } else {
        panic!("Cannot concat aggregated type!")
      }
    }
    assert!(result.len() == lhs.data.len(), "Width of the concat result does not match!");

    let slice = result.as_mut_bitslice();
    slice.reverse();
    let data = StateData::new_bits(BitVec::from_bitslice(slice), lhs.signed);
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for CombConcatEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CombConcatEvent")
      .field("lhs", &self.lhs)
      .field("ops", &self.ops)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct CombExtractEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  rhs: StateId,
  low: usize,
  high: usize,
}

impl SimEvent for CombExtractEvent {
  fn run(&self) {
    let StateData::Bits(lhs) = self.lhs.read_from(&self.container) else {
      panic!("Cannot extract from aggregated type!")
    };
    let StateData::Bits(rhs) = self.rhs.read_from(&self.container) else {
      panic!("Cannot extract from aggregated type!")
    };

    let data = StateData::new_bits(
      BitVec::from_bitslice(&rhs.data[self.low..self.high]),
      lhs.signed,
    );
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for CombExtractEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CombExtractEvent")
      .field("lhs", &self.lhs)
      .field("rhs", &self.rhs)
      .field("range", &(self.low, self.high))
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct ArrayConcatEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  ops: Vec<StateId>,
}

impl SimEvent for ArrayConcatEvent {
  fn run(&self) {
    let mut result = Vec::new();
    for op in &self.ops {
      let x = op.read_from(&self.container);
      if let StateData::Aggregate(y) = x {
        result.extend(y.children.iter().map(|x| *x.clone()))
      } else {
        panic!("Cannot array concat bits type!")
      }
    }

    let data = StateData::new_aggregate(result.into_iter());
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for ArrayConcatEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArrayConcatEvent")
      .field("lhs", &self.lhs)
      .field("ops", &self.ops)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct ArrayCreateEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  ops: Vec<StateId>,
}

impl SimEvent for ArrayCreateEvent {
  fn run(&self) {
    let mut result = Vec::new();
    for op in &self.ops {
      let x = op.read_from(&self.container);
      result.push(x);
    }

    let data = StateData::new_aggregate(result.into_iter());
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for ArrayCreateEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArrayCreateEvent")
      .field("lhs", &self.lhs)
      .field("ops", &self.ops)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct ArrayGetEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  array: StateId,
  index: StateId,
}

impl SimEvent for ArrayGetEvent {
  fn run(&self) {
    let StateData::Aggregate(array) = self.array.read_from(&self.container) else {
      panic!("Cannot apply array get on bits!")
    };
    let StateData::Bits(index) = self.index.read_from(&self.container) else {
      panic!("Cannot use aggregated type as index!")
    };
    let index: usize = index.data.toBigInt(index.signed).try_into().unwrap();

    let data = *array.children.get(index).unwrap().clone();
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for ArrayGetEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArrayGetEvent")
      .field("lhs", &self.lhs)
      .field("array", &self.array)
      .field("index", &self.index)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct ArrayConstGetEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  array: StateId,
  index: usize,
}

impl SimEvent for ArrayConstGetEvent {
  fn run(&self) {
    let StateData::Aggregate(array) = self.array.read_from(&self.container) else {
      panic!("Cannot apply array get on bits!")
    };

    let data = *array.children.get(self.index).unwrap().clone();
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for ArrayConstGetEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArrayGetEvent")
      .field("lhs", &self.lhs)
      .field("array", &self.array)
      .field("index", &self.index)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct ArraySliceEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  array: StateId,
  index: StateId,
  width: usize,
}

impl SimEvent for ArraySliceEvent {
  fn run(&self) {
    let StateData::Aggregate(array) = self.array.read_from(&self.container) else {
      panic!("Cannot apply array slice on bits!")
    };
    let StateData::Bits(index) = self.index.read_from(&self.container) else {
      panic!("Cannot use aggregated type as index!")
    };
    let index: usize = index.data.toBigInt(index.signed).try_into().unwrap();

    let data = StateData::new_aggregate(
      array.children[index..index + self.width].iter().map(|x| *x.clone()),
    );
    self.lhs.write_to(data, &self.container);
  }
}

impl Debug for ArraySliceEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ArraySliceEvent")
      .field("lhs", &self.lhs)
      .field("array", &self.array)
      .field("index", &self.index)
      .field("width", &self.width)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct StructInjectEvent {
  container: Arc<RwLock<SimStateContainer>>,
  lhs: StateId,
  rhs: StateId,
  value: StateId,
  index: usize,
}

impl SimEvent for StructInjectEvent {
  fn run(&self) {
    let StateData::Aggregate(mut rhs) = self.rhs.read_from(&self.container) else {
      panic!("Cannot apply struct inject on bits!")
    };
    let value = self.value.read_from(&self.container);
    rhs.children[self.index] = Box::new(value);
    self.lhs.write_to(StateData::Aggregate(rhs), &self.container);
  }
}

impl Debug for StructInjectEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("StructInjectEvent")
      .field("lhs", &self.lhs)
      .field("rhs", &self.rhs)
      .field("value", &self.value)
      .field("index", &self.index)
      .finish()
  }
}

#[StructFields(pub)]
#[derive(Clone)]
pub struct StructExplodeEvent {
  container: Arc<RwLock<SimStateContainer>>,
  outputs: Vec<StateId>,
  rhs: StateId,
}

impl SimEvent for StructExplodeEvent {
  fn run(&self) {
    let StateData::Aggregate(rhs) = self.rhs.read_from(&self.container) else {
      panic!("Cannot apply struct inject on bits!")
    };
    assert!(rhs.children.len() == self.outputs.len());
    for (data, lhs) in rhs.children.into_iter().zip(self.outputs.iter()) {
      lhs.write_to(*data, &self.container);
    }
  }
}

impl Debug for StructExplodeEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("StructExplodeEvent")
      .field("exploded", &self.outputs)
      .field("rhs", &self.rhs)
      .finish()
  }
}
