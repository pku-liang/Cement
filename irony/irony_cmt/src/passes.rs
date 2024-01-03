use core::panic;
use std::collections::HashMap;

use irony::{Entity, Environ, Op, OpId, PassManagerTrait, PassTrait};

use crate::{
  Assign, AttributeEnum, CombBinaryPredicate, CombMux2, CombVariadicPredicate,
  ConstantAttr, EntityEnum, EventSignal, HwConstant, IRWire, OpEnum, StringAttr,
  SvConstantX, TmpSelect, TmpUnary, TmpWhen,
};

#[derive(Debug, Clone)]
pub struct ReorderPass;

impl Into<PassEnum> for ReorderPass {
  fn into(self) -> PassEnum { PassEnum::ReorderPass(self) }
}

impl PassTrait<(), ()> for ReorderPass {
  type EntityT = EntityEnum;
  type OpT = OpEnum;

  fn check_op<E>(&self, env: &E, op: OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match env.get_op(op) {
      OpEnum::HwModule(_) => true,
      _ => false,
    }
  }

  fn run_raw<E>(&self, env: &mut E, op: OpId) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    let region = env.get_op(op).get_regions()[0].1[0].expect("must have region");

    let included = env.get_region(region).op_children.to_owned();
    let mut head = Vec::new();
    let mut body = Vec::new();
    let mut tail = Vec::new();

    for op_id in included {
      match env.get_op(op_id) {
        OpEnum::HwInput(_)
        | OpEnum::EventDef(_)
        | OpEnum::EventPort(_)
        | OpEnum::EventSignal(_) => {
          head.push(op_id);
        },
        OpEnum::HwOutput(_) => {
          tail.push(op_id);
        },
        _ => {
          body.push(op_id);
        },
      }
    }
    env.get_region_entry(region).and_modify(|region| {
      region.op_children =
        head.into_iter().chain(body.into_iter()).chain(tail.into_iter()).collect();
    });

    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct RemoveEventPass;

impl Into<PassEnum> for RemoveEventPass {
  fn into(self) -> PassEnum { PassEnum::RemoveEventPass(self) }
}

impl PassTrait<(), ()> for RemoveEventPass {
  type EntityT = EntityEnum;
  type OpT = OpEnum;

  fn check_op<E>(&self, env: &E, op: OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match env.get_op(op) {
      OpEnum::HwModule(_) => true,
      _ => false,
    }
  }

  fn run_raw<E>(&self, env: &mut E, op: OpId) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    let mut event_signal_mapping: HashMap<irony::EntityId, Vec<irony::EntityId>> =
      HashMap::new();
    let mut wire_guarded_table = HashMap::new();
    let mut wire_to_be_selected_table = HashMap::new();
    let region = env.get_op(op).get_regions()[0].1[0].expect("must have region");
    let included = env.get_region(region).op_children.to_owned();
    let mut new_included = Vec::new();

    for op_id in included.iter() {
      let op = env.get_op(op_id.to_owned()).to_owned();
      match op {
        OpEnum::EventSignal(EventSignal {
          event: Some(event),
          signal: Some(signal),
          ..
        }) => {
          event_signal_mapping
            .entry(event.to_owned())
            .or_insert(Vec::new())
            .push(signal.to_owned());
        },
        OpEnum::TmpSelect(TmpSelect { conds, values, .. }) => {
          for (cond, value) in conds.iter().zip(values.iter()) {
            let value_id = value.unwrap().to_owned();
            if let None = cond.to_owned() {
              *wire_to_be_selected_table.entry(value_id).or_insert(0) += 1;
            }
          }
        },
        _ => {},
      }
    }

    for (event, signals) in event_signal_mapping.into_iter() {
      let source_signals: Vec<_> = signals
        .to_owned()
        .into_iter()
        .filter(|&x| {
          let defs = env.get_defs(x.to_owned());
          !defs.is_empty()
        })
        .collect();
      assert!(
        source_signals.len() != 0,
        "{:?}({:?}) doesn't have source signals, which control synthesis should provide",
        event,
        env.get_entity(event).get_attr("name").unwrap()
      );
      assert!(source_signals.len() == 1, "event must have at most one source signal");
      let source_signal = source_signals[0].to_owned();
      let event_uses = env.get_uses(event.to_owned());
      for event_use in event_uses {
        env
          .get_op_entry(event_use)
          .and_modify(|op| op.replace_use(event.to_owned(), source_signal.to_owned()));
      }
      for signal in signals {
        if signal == source_signal {
          continue;
        }
        let signal_uses = env.get_uses(signal.to_owned());
        for signal_use in signal_uses {
          env
            .get_op_entry(signal_use)
            .and_modify(|op| op.replace_use(signal.to_owned(), source_signal.to_owned()));
        }
      }
    }

    for op_id in included.iter() {
      let op = env.get_op(op_id.to_owned()).to_owned();
      match op {
        OpEnum::EventDef(_) | OpEnum::EventPort(_) | OpEnum::EventSignal(_) => {
          env.delete_op(op_id.to_owned());
        },
        OpEnum::TmpWhen(TmpWhen { cond: Some(cond), body: Some(body), .. }) => {
          let body = env.get_region(body.to_owned()).op_children.to_owned();
          for op_id in body {
            let defs = env
              .get_op(op_id)
              .get_defs()
              .into_iter()
              .flat_map(|(_, v)| v.into_iter().filter_map(|x| x.map(|x| x.to_owned())))
              .collect::<Vec<_>>();

            let mut only_to_be_selected = true;
            for def in defs {
              wire_guarded_table.insert(def.to_owned(), cond.to_owned());
              let times_be_used = env.get_uses(def.to_owned()).len();
              let times_to_be_selected = wire_to_be_selected_table
                .get(&def)
                .to_owned()
                .map(|x| x.to_owned())
                .unwrap_or(0);
              only_to_be_selected &= times_be_used == times_to_be_selected;
            }

            if only_to_be_selected {
              new_included.push(op_id.to_owned());
              env.get_op_entry(op_id).and_modify(|op| {
                op.set_parent(Some(region));
              });
            } else {
              panic!("defs in when body must be only used in select")
            }
          }

          env.delete_op(op_id.to_owned());
        },
        _ => {
          new_included.push(op_id.to_owned());
        },
      }
    }

    for op_id in new_included.iter() {
      let op = env.get_op(op_id.to_owned()).to_owned();
      match op {
        OpEnum::TmpSelect(TmpSelect { conds, values, .. }) => {
          let mut new_conds = Vec::new();
          for (old_cond, value) in conds.iter().zip(values.iter()) {
            let value_id = value.unwrap().to_owned();
            if let None = old_cond.to_owned() {
              let cond_id = wire_guarded_table
                .get(&value_id)
                .expect("value to be selected must be guarded")
                .to_owned();
              let cond = env.get_entity(cond_id).to_owned();
              let signal_id = match cond {
                EntityEnum::IRWire(_) => cond_id,
                _ => {
                  panic!()
                },
              };
              new_conds.push(Some(signal_id));
            } else {
              new_conds.push(old_cond.to_owned());
            }
          }
          env.get_op_entry(op_id.to_owned()).and_modify(|op| match op {
            OpEnum::TmpSelect(TmpSelect { conds, .. }) => {
              *conds = new_conds;
            },
            _ => {
              panic!()
            },
          });
        },
        _ => {},
      }
    }

    env.get_region_entry(region).and_modify(|region| {
      region.op_children = new_included;
    });

    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct RemoveSelectPass;

impl Into<PassEnum> for RemoveSelectPass {
  fn into(self) -> PassEnum { PassEnum::RemoveSelectPass(self) }
}

impl PassTrait<(), ()> for RemoveSelectPass {
  type EntityT = EntityEnum;
  type OpT = OpEnum;

  fn check_op<E>(&self, env: &E, op: OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match env.get_op(op) {
      OpEnum::HwModule(_) => true,
      _ => false,
    }
  }

  fn run_raw<E>(&self, env: &mut E, op: OpId) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    let region = env.get_op(op).get_regions()[0].1[0].expect("must have region");
    let included = env.get_region(region).op_children.to_owned();
    let mut included_entity = env.get_region(region).entity_children.to_owned();
    let mut new_included_op = Vec::new();

    for op_id in included.iter() {
      let op = env.get_op(op_id.to_owned()).to_owned();
      match op {
        OpEnum::TmpSelect(TmpSelect { lhs, conds, values, default, onehot, .. }) => {
          let onehot = onehot.unwrap_or(false.into()).0;
          if onehot {
            println!("[TODO] support onehot select");
          }
          let onehot = false;
          if onehot {

            //TODO: support onehot select
          } else {
            let default = match default {
              Some(default) => default,
              None => {
                let data_type = env
                  .get_entity(
                    values
                      .first()
                      .expect("must have at least one value")
                      .expect("must be Some")
                      .to_owned(),
                  )
                  .get_dtype();
                let AttributeEnum::StringAttr(StringAttr(name)) = env
                  .get_entity(lhs.expect("must be Some").to_owned())
                  .get_attr("name")
                  .unwrap()
                else {
                  panic!()
                };
                let AttributeEnum::LocationAttr(location) = env
                  .get_entity(lhs.expect("must be Some").to_owned())
                  .get_attr("location")
                  .unwrap()
                else {
                  panic!()
                };
                let AttributeEnum::BoolAttr(debug) = env
                  .get_entity(lhs.expect("must be Some").to_owned())
                  .get_attr("debug")
                  .unwrap()
                else {
                  panic!()
                };

                // unimplemented!()
                let default = env.add_entity(
                  {
                    let mut wire = IRWire::new(
                      data_type,
                      Some((name + "_default").into()),
                      Some(debug),
                      Some(location),
                    );
                    wire.parent = Some(region.to_owned());
                    wire
                  }
                  .into(),
                );

                included_entity.push(default.to_owned());

                let constant = env.add_op(
                  {
                    let mut op = SvConstantX::new(Some(default));
                    op.parent = Some(region.to_owned());
                    op
                  }
                  .into(),
                );

                new_included_op.push(constant);
                default
              },
            };

            let mut last = default;
            for (cond, value) in conds.iter().zip(values.iter()).rev() {
              let cond = cond.to_owned().expect("must be Some");
              let value = value.to_owned().expect("must be Some");
              let data_type = env.get_entity(value).get_dtype();
              let AttributeEnum::StringAttr(StringAttr(name)) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("name")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::LocationAttr(location) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("location")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::BoolAttr(debug) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("debug")
                .unwrap()
              else {
                panic!()
              };

              let mux_wire = env.add_entity(
                {
                  let mut wire = IRWire::new(
                    data_type,
                    Some((name + "_mux").into()),
                    Some(debug),
                    Some(location),
                  );
                  wire.parent = Some(region.to_owned());
                  wire
                }
                .into(),
              );

              included_entity.push(mux_wire.to_owned());

              let mux_op = env.add_op(
                {
                  let mut mux = CombMux2::new(
                    Some(mux_wire.to_owned()),
                    Some(cond),
                    Some(value),
                    Some(last),
                  );
                  mux.parent = Some(region.to_owned());
                  mux
                }
                .into(),
              );

              new_included_op.push(mux_op);

              last = mux_wire;
            }

            let assign = env.add_op(
              {
                let mut assign =
                  Assign::new(Some(lhs.expect("must be Some")), Some(last));
                assign.parent = Some(region.to_owned());
                assign
              }
              .into(),
            );

            new_included_op.push(assign);
          }
          env.delete_op(op_id.to_owned());
        },
        _ => {
          new_included_op.push(op_id.to_owned());
        },
      }
    }

    env.get_region_entry(region).and_modify(|region| {
      region.op_children = new_included_op;
      region.entity_children = included_entity;
    });

    Ok(())
  }
}

#[derive(Debug, Clone)]
pub struct RemoveUnaryPass;
impl Into<PassEnum> for RemoveUnaryPass {
  fn into(self) -> PassEnum { PassEnum::RemoveUnaryPass(self) }
}
impl PassTrait<(), ()> for RemoveUnaryPass {
  type EntityT = EntityEnum;
  type OpT = OpEnum;

  fn check_op<E>(&self, env: &E, op: OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match env.get_op(op) {
      OpEnum::HwModule(_) => true,
      _ => false,
    }
  }

  fn run_raw<E>(&self, env: &mut E, op: OpId) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    let region = env.get_op(op).get_regions()[0].1[0].expect("must have region");
    let included = env.get_region(region).op_children.to_owned();
    let mut new_included_op = Vec::new();
    let mut included_entity = env.get_region(region).entity_children.to_owned();

    for op_id in included.iter() {
      let op = env.get_op(op_id.to_owned()).to_owned();
      match op {
        OpEnum::TmpUnary(TmpUnary { lhs, op, predicate, .. }) => {
          match predicate.expect("unary predicate must be some") {
            crate::CombUnaryPredicate::Not => {
              let AttributeEnum::StringAttr(StringAttr(name)) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("name")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::LocationAttr(location) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("location")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::BoolAttr(debug) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("debug")
                .unwrap()
              else {
                panic!()
              };
              let data_type =
                env.get_entity(lhs.expect("must be Some").to_owned()).get_dtype();
              let one = env.add_entity(
                {
                  let mut wire = IRWire::new(
                    data_type,
                    Some((name + "_one").into()),
                    Some(debug),
                    Some(location),
                  );
                  wire.parent = Some(region.to_owned());
                  wire
                }
                .into(),
              );
              included_entity.push(one.to_owned());
              let constant_op = env.add_op(
                {
                  let mut op =
                    HwConstant::new(Some(one.to_owned()), Some(ConstantAttr(vec![true])));
                  op.parent = Some(region.to_owned());
                  op
                }
                .into(),
              );
              new_included_op.push(constant_op);
              let xor_op = env.add_op(
                {
                  let mut op = crate::CombVariadic::new(
                    Some(lhs.expect("must be Some").to_owned()),
                    vec![
                      Some(op.expect("must be Some").to_owned()),
                      Some(one.to_owned()),
                    ],
                    Some(CombVariadicPredicate::Xor),
                  );
                  op.parent = Some(region.to_owned());
                  op
                }
                .into(),
              );
              new_included_op.push(xor_op);
              env.delete_op(op_id.to_owned());
            },
            crate::CombUnaryPredicate::Neg => {
              let AttributeEnum::StringAttr(StringAttr(name)) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("name")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::LocationAttr(location) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("location")
                .unwrap()
              else {
                panic!()
              };
              let AttributeEnum::BoolAttr(debug) = env
                .get_entity(lhs.expect("must be Some").to_owned())
                .get_attr("debug")
                .unwrap()
              else {
                panic!()
              };
              let data_type =
                env.get_entity(lhs.expect("must be Some").to_owned()).get_dtype();
              let zero = env.add_entity(
                {
                  let mut wire = IRWire::new(
                    data_type,
                    Some((name + "_zero").into()),
                    Some(debug),
                    Some(location),
                  );
                  wire.parent = Some(region.to_owned());
                  wire
                }
                .into(),
              );
              included_entity.push(zero.to_owned());
              let constant_op = env.add_op(
                {
                  let mut op = HwConstant::new(
                    Some(zero.to_owned()),
                    Some(ConstantAttr(vec![false])),
                  );
                  op.parent = Some(region.to_owned());
                  op
                }
                .into(),
              );
              new_included_op.push(constant_op);
              let sub_op = env.add_op(
                {
                  let mut op = crate::CombBinary::new(
                    Some(lhs.expect("must be Some").to_owned()),
                    Some(zero.to_owned()),
                    Some(lhs.expect("must be Some").to_owned()),
                    Some(CombBinaryPredicate::Sub),
                  );
                  op.parent = Some(region.to_owned());
                  op
                }
                .into(),
              );
              new_included_op.push(sub_op);
              env.delete_op(op_id.to_owned());
            },
          }
        },
        _ => {
          new_included_op.push(op_id.to_owned());
        },
      }
    }

    env.get_region_entry(region).and_modify(|region| {
      region.op_children = new_included_op;
      region.entity_children = included_entity;
    });

    Ok(())
  }
}

#[derive(Debug, Clone)]
pub enum PassEnum {
  ReorderPass(ReorderPass),
  RemoveEventPass(RemoveEventPass),
  RemoveSelectPass(RemoveSelectPass),
  RemoveUnaryPass(RemoveUnaryPass),
}

impl PassTrait<(), ()> for PassEnum {
  type EntityT = EntityEnum;
  type OpT = OpEnum;

  fn check_op<E>(&self, env: &E, op_id: irony::OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match self {
      PassEnum::ReorderPass(pass) => pass.check_op(env, op_id),
      PassEnum::RemoveEventPass(pass) => pass.check_op(env, op_id),
      PassEnum::RemoveSelectPass(pass) => pass.check_op(env, op_id),
      PassEnum::RemoveUnaryPass(pass) => pass.check_op(env, op_id),
    }
  }

  fn run_raw<E>(&self, env: &mut E, op_id: irony::OpId) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    match self {
      PassEnum::ReorderPass(pass) => pass.run_raw(env, op_id),
      PassEnum::RemoveEventPass(pass) => pass.run_raw(env, op_id),
      PassEnum::RemoveSelectPass(pass) => pass.run_raw(env, op_id),
      PassEnum::RemoveUnaryPass(pass) => pass.run_raw(env, op_id),
    }
  }
}

#[derive(Default, Debug, Clone)]
pub struct PassManager {
  passes: Vec<PassEnum>,
  start_ops: Vec<Vec<OpId>>,
}

impl PassManagerTrait<(), ()> for PassManager {
  type EntityT = EntityEnum;
  type OpT = OpEnum;
  type PassT = PassEnum;

  fn add_passes(&mut self, mut passes: Vec<Self::PassT>, mut start_ops: Vec<Vec<OpId>>) {
    assert_eq!(passes.len(), start_ops.len());
    self.passes.append(&mut passes);
    self.start_ops.append(&mut start_ops);
  }

  fn run_passes<E>(&self, env: &mut E) -> Result<(), ()>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    for (pass, op) in self.passes.iter().zip(self.start_ops.iter()) {
      for op in op.iter() {
        pass.run_on(env, *op)?;
      }
    }
    Ok(())
  }
}
