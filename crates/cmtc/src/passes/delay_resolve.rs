use builtin::{fifo1_gen, shift_reg_gen};
use indexmap::IndexSet;

use crate::{DelayOp, DynDelayOp};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DelayHandle {
  inst_rule: InstRule,
  delay: u32,
  is_dyn: bool,
}

pub struct DelayResolvePass {
  delay_table: IndexSet<DelayHandle>,
  delay_update_in_body: Option<ir::Op>,
}

impl DelayResolvePass {
  pub fn new() -> Self {
    Self {
      delay_table: IndexSet::new(),
      delay_update_in_body: None,
    }
  }
}

impl Visitor for DelayResolvePass {
  fn name() -> &'static str {
    "delay-resolve"
  }

  /// tb module should have no delay
  fn skip_tb() -> bool {
    true
  }

  fn visit_rule_guard(&mut self, data: &mut VisitorData) -> anyhow::Result<()> {
    self.delay_update_in_body = None;
    let mut guard_ops = std::mem::take(&mut data.rule_mut().guard_ops);
    for op in guard_ops.iter_mut() {
      let new_op = match op.inner() {
        ir::OpEnum::Delay(DelayOp {
          res,
          inst_rule,
          delay,
        }) => {
          let delay = DelayHandle {
            inst_rule: inst_rule.clone(),
            delay: *delay,
            is_dyn: false,
          };
          self.delay_table.insert(delay.clone());
          Some(delay_use(*res, delay, data)?)
        }
        ir::OpEnum::DynDelay(DynDelayOp {
          res,
          inst_rule,
          delay,
        }) => {
          let delay = DelayHandle {
            inst_rule: inst_rule.clone(),
            delay: *delay,
            is_dyn: true,
          };
          self.delay_table.insert(delay.clone());
          self.delay_update_in_body = delay_update(&delay, data)?;
          Some(delay_use(*res, delay, data)?)
        }
        _ => None,
      };

      if let Some(new_op) = new_op {
        *op = new_op;
      }
    }
    std::mem::swap(&mut data.rule_mut().guard_ops, &mut guard_ops);
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    if data.module.is_ext() {
      return Ok((vec![data.take_rule()], vec![]));
    }

    if let Some(delay_update) = self.delay_update_in_body.take() {
      data.rule_mut().ops.insert(0, delay_update);
    }

    Ok((vec![data.take_rule()], vec![]))
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    if data.module.is_ext() {
      return Ok(());
    }

    for handle in self.delay_table.iter() {
      if !handle.inst_rule.is_self() {
        anyhow::bail!(data.report_error(format!(
          "delay inst must be self, but {}",
          handle.inst_rule.path.join(".")
        )));
      }

      // insert delay ops according  to delay_table
      let lit1 = data.module.add_value(None, Some(ir::Type::UInt(1)));
      let delay_set_op = delay_set(handle, lit1, data)?;
      let error = data
        .report_error(format!("rule {} not found", handle.inst_rule.rule_name));
      let rule = data
        .rule_mut_from_rule_name(&handle.inst_rule.rule_name)
        .ok_or(error)?;
      rule.ops.insert(0, delay_set_op);
      rule.ops.insert(0, ir::Op::lit_int(1, 1, lit1));

      // insert instances
      add_delay_inst(handle, data)?;
    }
    self.delay_table.clear();
    Ok(())
  }
}

fn delay_inst(
  handle: &DelayHandle,
  data: &mut VisitorData,
) -> anyhow::Result<String> {
  if !handle.inst_rule.is_self() {
    anyhow::bail!(data.report_error(format!(
      "delay inst must be self, but {}",
      handle.inst_rule.path.join(".")
    )));
  }
  Ok(format!(
    "{}delay_{}_by_{}",
    if handle.is_dyn { "dyn_" } else { "" },
    handle.inst_rule.rule_name,
    handle.delay
  ))
}

fn delay_use(
  res: ir::ValueId,
  delay: DelayHandle,
  data: &mut VisitorData,
) -> anyhow::Result<ir::Op> {
  Ok(ir::Op::invoke(
    ir::InstRule::new(vec![
      delay_inst(&delay, data)?,
      if !delay.is_dyn { "read" } else { "full" }.to_string(),
    ]),
    vec![],
    vec![res],
  ))
}

fn delay_set(
  handle: &DelayHandle,
  lit: ir::ValueId,
  data: &mut VisitorData,
) -> anyhow::Result<ir::Op> {
  Ok(ir::Op::invoke(
    ir::InstRule::new(vec![
      delay_inst(handle, data)?,
      if !handle.is_dyn { "write" } else { "enq" }.to_string(),
    ]),
    vec![lit],
    vec![],
  ))
}

fn add_delay_inst(
  handle: &DelayHandle,
  data: &mut VisitorData,
) -> anyhow::Result<()> {
  let inst_name = delay_inst(handle, data)?;
  let modules = if handle.is_dyn {
    fifo1_gen(ir::Type::UInt(1))
  } else {
    shift_reg_gen(ir::Type::UInt(1), handle.delay as usize)
  };

  let module_name = modules
    .last()
    .ok_or(
      data.report_error(format!("delay inst {} has no module", inst_name)),
    )?
    .name
    .clone();

  for module in modules {
    data.circuit.add_module(module);
  }

  let inst_def = ir::InstDef::new(inst_name, module_name);

  data.module.instances_mut().push(inst_def);
  Ok(())
}

fn delay_update(
  delay: &DelayHandle,
  data: &mut VisitorData,
) -> anyhow::Result<Option<ir::Op>> {
  if delay.is_dyn {
    let deq_value = data.module.add_value(None, Some(ir::Type::UInt(1)));
    Ok(Some(ir::Op::invoke(
      ir::InstRule::new(vec![delay_inst(delay, data)?, "deq".to_string()]),
      vec![],
      vec![deq_value],
    )))
  } else {
    Ok(None)
  }
}