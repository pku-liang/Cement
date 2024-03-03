use core::panic;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use bitvec::prelude as bv;
use irony_cmt::{
  ArrayAttr, AttributeEnum, DataTypeEnum, EntityEnum, EntityId, Environ, HwInstance, Op,
  OpEnum, OpId,
};

use super::events::*;
use super::schedule::SimCycle;
use super::state::*;
use super::{CmtIR, Cmtc};
use crate::utils::*;

pub(super) fn get_top(dut: &Cmtc) -> OpId {
  for (id, op) in dut.ir.op_table.iter() {
    if let OpEnum::HwModule(module) = op {
      if module.top.as_ref().unwrap().0 {
        return OpId(*id);
      }
    }
  }
  panic!("Top module not found!")
}

pub(super) fn make_simulator(
  dut: &Cmtc, module: OpId, container: &Arc<RwLock<SimStateContainer>>,
) -> (SimCycle, Vec<StateId>, Vec<StateId>) {
  let mut inputs = Vec::new();
  let mut outputs = Vec::new();
  let mut cycle = SimCycle::empty();
  let state_table = alloc_local_states(&dut.ir, module, container);

  let OpEnum::HwModule(module) = dut.ir.get_op(module) else { panic!() };
  let region = dut.ir.get_region(module.body.unwrap());
  let ops = top_sort(&dut.ir, region.get_op_children());

  for id in &ops {
    let op = dut.ir.get_op(*id);
    match op {
      OpEnum::Assign(assign) => cycle.comb_events.push(Box::new(AssignEvent {
        container: Arc::clone(container),
        lhs: state_table[assign.lhs.as_ref().unwrap()],
        rhs: state_table[assign.rhs.as_ref().unwrap()],
      })),
      OpEnum::HwModule(_) => panic!("Nested module declaration is not supported"),
      OpEnum::HwInstance(instance) => {
        cycle.merge(instance_submodule(dut, instance, container, &state_table))
      },
      OpEnum::HwInput(input) => inputs
        .extend(input.inputs.iter().filter_map(|x| *x).map(|x| state_table[&x].clone())),
      OpEnum::HwOutput(output) => outputs.extend(
        output.outputs.iter().filter_map(|x| *x).map(|x| state_table[&x].clone()),
      ),
      OpEnum::HwBitCast(bitcast) => cycle.comb_events.push(Box::new(CastEvent {
        container: Arc::clone(container),
        lhs: state_table[bitcast.lhs.as_ref().unwrap()],
        rhs: state_table[bitcast.rhs.as_ref().unwrap()],
      })),
      OpEnum::HwConstant(constant) => {
        let lhs = constant.lhs.as_ref().unwrap();
        let id = &state_table[lhs];

        // TODO: if SInt is added, modify here!
        // let EntityEnum::IRWire(wire) = dut.ir.get_entity(*lhs) else {
        //   panic!("HwConstant lhs is not a wire!")
        // };
        let signed = false;

        let data = StateData::new_bool_vec(&constant.value.as_ref().unwrap().0, signed);
        id.write_to(data, container);
      },
      OpEnum::HwAggregateConstant(agg_constant) => {
        let data = StateData::new_aggregate(
          agg_constant.attrs.as_ref().unwrap().0.iter().map(|x| build_aggrated_const(x)),
        );
        let lhs = state_table[agg_constant.lhs.as_ref().unwrap()];
        lhs.write_to(data, container);
      },
      OpEnum::HwArrayConcat(array_concat) => {
        cycle.comb_events.push(Box::new(ArrayConcatEvent {
          container: Arc::clone(container),
          lhs: state_table[array_concat.lhs.as_ref().unwrap()],
          ops: array_concat
            .operands
            .iter()
            .map(|x| state_table[x.as_ref().unwrap()])
            .collect(),
        }))
      },
      OpEnum::HwArrayCreate(array_create) => {
        cycle.comb_events.push(Box::new(ArrayCreateEvent {
          container: Arc::clone(container),
          lhs: state_table[array_create.lhs.as_ref().unwrap()],
          ops: array_create
            .operands
            .iter()
            .map(|x| state_table[x.as_ref().unwrap()])
            .collect(),
        }))
      },
      OpEnum::HwArrayGet(array_get) => cycle.comb_events.push(Box::new(ArrayGetEvent {
        container: Arc::clone(container),
        lhs: state_table[array_get.lhs.as_ref().unwrap()],
        array: state_table[array_get.array.as_ref().unwrap()],
        index: state_table[array_get.index.as_ref().unwrap()],
      })),
      OpEnum::HwArraySlice(array_slice) => {
        cycle.comb_events.push(Box::new(ArraySliceEvent {
          container: Arc::clone(container),
          lhs: state_table[array_slice.lhs.as_ref().unwrap()],
          array: state_table[array_slice.array.as_ref().unwrap()],
          index: state_table[array_slice.index.as_ref().unwrap()],
          width: get_array_width(&dut.ir, array_slice.lhs.unwrap()),
        }))
      },
      OpEnum::HwStructCreate(struct_create) => {
        cycle.comb_events.push(Box::new(ArrayCreateEvent {
          container: Arc::clone(container),
          lhs: state_table[struct_create.lhs.as_ref().unwrap()],
          ops: struct_create
            .operands
            .iter()
            .map(|x| state_table[x.as_ref().unwrap()])
            .collect(),
        }))
      },
      OpEnum::HwStructExtract(struct_extract) => {
        cycle.comb_events.push(Box::new(ArrayConstGetEvent {
          container: Arc::clone(container),
          lhs: state_table[struct_extract.lhs.as_ref().unwrap()],
          array: state_table[struct_extract.struct_input.as_ref().unwrap()],
          index: get_field_index(
            &dut.ir,
            struct_extract.struct_input.unwrap(),
            &struct_extract.field.as_ref().unwrap().0,
          ),
        }))
      },
      OpEnum::HwStructInject(struct_inject) => {
        cycle.comb_events.push(Box::new(StructInjectEvent {
          container: Arc::clone(container),
          lhs: state_table[struct_inject.lhs.as_ref().unwrap()],
          rhs: state_table[struct_inject.struct_input.as_ref().unwrap()],
          value: state_table[struct_inject.new_value.as_ref().unwrap()],
          index: get_field_index(
            &dut.ir,
            struct_inject.struct_input.unwrap(),
            &struct_inject.field.as_ref().unwrap().0,
          ),
        }))
      },
      OpEnum::HwStructExplode(struct_explode) => {
        cycle.comb_events.push(Box::new(StructExplodeEvent {
          container: Arc::clone(container),
          outputs: struct_explode
            .outputs
            .iter()
            .map(|x| state_table[x.as_ref().unwrap()])
            .collect(),
          rhs: state_table[struct_explode.struct_input.as_ref().unwrap()],
        }))
      },
      OpEnum::CombVariadic(variadic) => {
        cycle.comb_events.push(Box::new(CombVariadicEvent {
          container: Arc::clone(container),
          lhs: state_table[variadic.lhs.as_ref().unwrap()],
          ops: variadic
            .operands
            .iter()
            .map(|x| state_table[x.as_ref().unwrap()])
            .collect(),
          predicate: variadic.predicate.clone().unwrap(),
        }))
      },
      OpEnum::CombBinary(binary) => cycle.comb_events.push(Box::new(CombBinaryEvent {
        container: Arc::clone(container),
        lhs: state_table[binary.lhs.as_ref().unwrap()],
        op0: state_table[binary.op0.as_ref().unwrap()],
        op1: state_table[binary.op1.as_ref().unwrap()],
        predicate: binary.predicate.clone().unwrap(),
      })),
      OpEnum::CombICmp(icmp) => cycle.comb_events.push(Box::new(CombICmpEvent {
        container: Arc::clone(container),
        lhs: state_table[icmp.lhs.as_ref().unwrap()],
        op0: state_table[icmp.op0.as_ref().unwrap()],
        op1: state_table[icmp.op1.as_ref().unwrap()],
        predicate: icmp.predicate.clone().unwrap(),
      })),
      OpEnum::CombExtract(extract) => {
        let width = get_width(&dut.ir, extract.lhs.unwrap());
        let low: usize = extract.low.as_ref().unwrap().0.try_into().unwrap();

        cycle.comb_events.push(Box::new(CombExtractEvent {
          container: Arc::clone(container),
          lhs: state_table[extract.lhs.as_ref().unwrap()],
          rhs: state_table[extract.input.as_ref().unwrap()],
          low,
          high: low + width,
        }))
      },
      OpEnum::CombConcat(concat) => cycle.comb_events.push(Box::new(CombConcatEvent {
        container: Arc::clone(container),
        lhs: state_table[concat.lhs.as_ref().unwrap()],
        ops: concat.operands.iter().map(|x| state_table[x.as_ref().unwrap()]).collect(),
      })),
      OpEnum::CombMux2(mux) => cycle.comb_events.push(Box::new(MuxAssignEvent {
        container: Arc::clone(container),
        lhs: state_table[mux.lhs.as_ref().unwrap()],
        sel: state_table[mux.cond.as_ref().unwrap()],
        rhs0: state_table[mux.op0.as_ref().unwrap()],
        rhs1: state_table[mux.op1.as_ref().unwrap()],
      })),
      OpEnum::SeqCompReg(seq) => {
        // TODO: add multiple clock support?
        if let Some(reset) = seq.reset {
          cycle.reg_events.push(Box::new(MuxAssignEvent {
            container: Arc::clone(container),
            lhs: state_table[seq.input.as_ref().unwrap()],
            sel: state_table[&reset],
            rhs0: state_table[seq.output.as_ref().unwrap()],
            rhs1: state_table[seq.reset_val.as_ref().unwrap()],
          }));
        } else {
          cycle.reg_events.push(Box::new(AssignEvent {
            container: Arc::clone(container),
            lhs: state_table[seq.input.as_ref().unwrap()],
            rhs: state_table[seq.output.as_ref().unwrap()],
          }))
        }
      },
      _ => {},
    }
  }

  (cycle, inputs, outputs)
}

fn instance_submodule(
  dut: &Cmtc, instance: &HwInstance, container: &Arc<RwLock<SimStateContainer>>,
  state_table: &HashMap<EntityId, StateId>,
) -> SimCycle {
  let (instance_cycle, instance_in, instance_out) =
    make_simulator(dut, instance.target_op_id.as_ref().unwrap().0, &container);

  let mut cycle = SimCycle::empty();
  // Assign instance inputs
  for (mod_in, ins_in) in instance.inputs.iter().zip(instance_in) {
    cycle.comb_events.push(Box::new(AssignEvent {
      container: Arc::clone(container),
      lhs: ins_in,
      rhs: state_table[&mod_in.unwrap()].clone(),
    }));
  }

  cycle.merge(instance_cycle);

  // Assign instance outputs
  for (mod_out, ins_out) in instance.outputs.iter().zip(instance_out) {
    cycle.comb_events.push(Box::new(AssignEvent {
      container: Arc::clone(container),
      lhs: state_table[&mod_out.unwrap()].clone(),
      rhs: ins_out,
    }));
  }

  cycle
}

fn make_state_data(dtype: &DataTypeEnum) -> StateData {
  match dtype {
    DataTypeEnum::Clk(_) => StateData::empty_bits(1, false),
    DataTypeEnum::UInt(x) => StateData::empty_bits(x.0, false),
    DataTypeEnum::Struct(x) => {
      StateData::new_aggregate(x.0.iter().map(|(_, x)| make_state_data(x)))
    },
    DataTypeEnum::Array(x) => {
      StateData::new_aggregate((0..x.1).map(|_| make_state_data(&x.0)))
    },
    DataTypeEnum::UArray(x) => {
      StateData::new_aggregate((0..x.1).map(|_| make_state_data(&x.0)))
    },
    DataTypeEnum::SeqHlmem(_) => {
      todo!("SeqHlmem is not implmented")
    },
    DataTypeEnum::Void => {
      panic!("Don't know what to do with void")
    },
  }
}

fn alloc_local_states(
  ir: &CmtIR, module: OpId, container: &Arc<RwLock<SimStateContainer>>,
) -> HashMap<EntityId, StateId> {
  let mut state_table = HashMap::new();
  let OpEnum::HwModule(module) = ir.get_op(module) else {
    panic!("Not");
  };
  let region = ir.get_region(module.body.unwrap());
  for id in region.get_entity_children().iter() {
    if let EntityEnum::IRWire(wire) = ir.get_entity(*id) {
      // println!("{} {}", id.0, wire.name.clone().unwrap().to_string());
      let dtype = wire.dtype.as_ref().unwrap();
      state_table.insert(*id, container.write().unwrap().alloc(make_state_data(dtype)));
    }
  }
  state_table
}

pub(super) fn get_io_name(
  ir: &CmtIR, top: OpId, inputs: Vec<StateId>, outputs: Vec<StateId>,
) -> HashMap<String, StateId> {
  let OpEnum::HwModule(module) = ir.get_op(top) else { panic!() };
  let mut result = HashMap::new();

  for (name, handle) in
    module.arg_names.as_ref().unwrap().0.iter().zip(inputs.into_iter())
  {
    let AttributeEnum::StringAttr(str) = name else { panic!() };
    result.insert(str.0.clone(), handle);
  }

  for (name, handle) in
    module.output_names.as_ref().unwrap().0.iter().zip(outputs.into_iter())
  {
    let AttributeEnum::StringAttr(str) = name else { panic!() };
    result.insert(str.0.clone(), handle);
  }

  result
}

fn top_sort(ir: &CmtIR, ops: Vec<OpId>) -> Vec<OpId> {
  let mut result = Vec::new();

  let mut undef_set = HashMap::new();
  let mut use_map = HashMap::new();
  let mut que = VecDeque::new();
  let mut visited = HashSet::new();
  let mut def_set = HashSet::new();

  for x in &ops {
    let op = ir.get_op(*x);
    let undef = undef_set.entry(*x).or_insert(HashSet::new());
    for (_, es) in op.get_uses() {
      for eopt in es {
        eopt.map(|e| {
          use_map.entry(e).or_insert(Vec::new()).push(*x);
          undef.insert(e);
        });
      }
    }
    for (_, es) in op.get_uses() {
      for eopt in es {
        eopt.map(|e| def_set.insert(e));
      }
    }
    if undef.len() == 0 {
      que.push_back(*x);
      visited.insert(*x);
    }
  }
  assert_eq!(use_map.len(), def_set.len());
  for e in use_map.keys() {
    if !def_set.contains(e) {
      panic!("An entity that is used is never defined!");
    }
  }

  while let Some(x) = que.pop_front() {
    result.push(x);
    let op = ir.get_op(x);
    // println!("{:?}", op);
    for (_, es) in op.get_defs() {
      for eopt in es {
        eopt.map(|e| {
          for y in &use_map[&e] {
            let undef = undef_set.get_mut(y).unwrap();
            undef.remove(&e);
            if undef.len() == 0 && !visited.contains(y) {
              que.push_back(*y);
              visited.insert(*y);
            }
          }
        });
      }
    }
  }

  for (e, s) in undef_set {
    if s.len() > 0 {
      println!("{:?} {:?}", ir.get_op(e), s);
    }
  }

  assert!(result.len() == ops.len());

  result
}
fn get_width(ir: &CmtIR, id: EntityId) -> usize {
  if let EntityEnum::IRWire(x) = ir.get_entity(id) {
    if let DataTypeEnum::UInt(arr) = x.dtype.as_ref().unwrap() {
      arr.0
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

fn get_array_width(ir: &CmtIR, id: EntityId) -> usize {
  if let EntityEnum::IRWire(x) = ir.get_entity(id) {
    if let DataTypeEnum::Array(arr) = x.dtype.as_ref().unwrap() {
      arr.1
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

fn get_field_index(ir: &CmtIR, id: EntityId, target: &str) -> usize {
  if let EntityEnum::IRWire(x) = ir.get_entity(id) {
    if let DataTypeEnum::Struct(y) = x.dtype.as_ref().unwrap() {
      y.0.iter().position(|(name, _)| name == target).unwrap()
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

fn build_aggrated_const(input: &AttributeEnum) -> StateData {
  match input {
    AttributeEnum::ConstantAttr(x) => StateData::new_bool_vec(&x.0, false),
    AttributeEnum::ArrayAttr(x) => {
      StateData::new_aggregate(x.0.iter().map(|y| build_aggrated_const(y)))
    },
    _ => panic!("Not supported type in aggreted constant!"),
  }
}
