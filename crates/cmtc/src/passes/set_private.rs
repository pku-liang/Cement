use std::collections::HashSet;

use super::*;
use crate::passes::Visitor;

pub struct SetPrivatePass {
  self_callee_rules: HashSet<String>,
}

impl SetPrivatePass {
  pub fn new() -> Self {
    Self {
      self_callee_rules: HashSet::new(),
    }
  }
}

impl Visitor for SetPrivatePass {
  fn name() -> &'static str {
    "set_private"
  }

  fn visit_rule_guard(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    for op in Self::flatten_ops(data.rule().guard_ops.iter()) {
      if let Some(InvokeOp { inst_rule, .. }) = op.as_invoke_op_ref() {
        if inst_rule.is_self() {
          self.self_callee_rules.insert(inst_rule.rule_name.clone());
        }
      }
    }
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    let ops = Self::flatten_ops(data.rule().ops());
    for op in ops {
      if let Some(InvokeOp { inst_rule, .. }) = op.as_invoke_op_ref() {
        if inst_rule.is_self() {
          self.self_callee_rules.insert(inst_rule.rule_name.clone());
        }
      }
    }

    Ok((vec![data.take_rule()], vec![]))
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let module_inputs = data.module.inputs().collect::<HashSet<_>>();
    let module_outputs = data.module.outputs().collect::<HashSet<_>>();
    for rule in data.module.rules_mut() {
      if self.self_callee_rules.contains(rule.name()) {
        if !rule.is_private() {
          log::warn!(
            "rule {} is called by other rules in the same module, but is not private; the pass set it private!",
            rule.name()
          );
          rule.set_private(true);
        }
      }
    }
    for rule in data.module.rules() {
      if self.self_callee_rules.contains(rule.name()) {
        if rule.inputs().iter().any(|id| module_inputs.contains(id))
          || rule.outputs().iter().any(|id| module_outputs.contains(id))
        {
          return Err(data.report_error(format!(
            "private rule {} has inputs or outputs, which is not allowed",
            rule.name()
          )))?;
        }
      }
    }
    Ok(())
  }
}
