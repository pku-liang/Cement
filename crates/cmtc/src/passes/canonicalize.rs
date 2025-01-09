use super::*;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub struct CanonicalizePass {}

impl CanonicalizePass {
  pub fn new() -> Self {
    Self {}
  }
}

impl Visitor for CanonicalizePass {
  fn name() -> &'static str {
    "CanonicalizePass"
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<(Vec<ir::Rule>, Vec<ir::RuleRel>)> {
    let guard_ops =
      data.rule().guard().map(|op| op.clone()).collect::<Vec<_>>();
    let mut value_map: HashMap<ir::ValueId, ir::ValueId> = HashMap::new();
    for op in guard_ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Assign(ir::AssignOp { res, value }) = op.inner() {
          if res.name(&data.module.values).is_none() {
            value_map.clear();
            value_map.insert(*value, *res);
            data.rule_mut().replace_all_op_with_map(&value_map);
            data.rule_mut().remove_unused_op();
          } else if value.name(&data.module.values).is_none() {
            value_map.clear();
            value_map.insert(*res, *value);
            data.rule_mut().replace_all_op_with_map(&value_map);
            data.rule_mut().remove_unused_op();
          }
        }
      }
    }

    let ops = data.rule().ops().map(|op| op.clone()).collect::<Vec<_>>();
    for op in ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Assign(ir::AssignOp { res, value }) = op.inner() {
          if res.name(&data.module.values).is_none() {
            value_map.clear();
            value_map.insert(*value, *res);
            data.rule_mut().replace_all_op_with_map(&value_map);
            data.rule_mut().remove_unused_op();
          } else if value.name(&data.module.values).is_none() {
            value_map.clear();
            value_map.insert(*res, *value);
            data.rule_mut().replace_all_op_with_map(&value_map);
            data.rule_mut().remove_unused_op();
          }
        }
      }
    }

    // Keep the rule as is
    Ok((vec![data.take_rule()], vec![]))
  }
}
