use petgraph::data;

use crate::TimingExpr;

use super::*;

impl<'modu> VisitorData<'modu> {
  pub fn rule_conflict(
    &self,
    module_name: String,
    lhs: String,
    rhs: String,
  ) -> ir::MethodRel {
    let rel_map = if let Some(rel_map) = self.rule_rel_map.get(&module_name) {
      rel_map
    } else {
      log::warn!("no rule rel map for module {}", module_name);
      panic!("no rule rel map for module {}", module_name);
    };
    let rel = rel_map
      .get(&(lhs.clone(), rhs.clone()))
      .unwrap_or(&ir::MethodRel::CF);
    // log::debug!("rule conflict: {} vs. {} : {}", lhs, rhs, rel.to_string());

    // if lhs=="start" && rhs == "update_3_idle" {

    // }

    return *rel;
  }

  fn conflict(
    &mut self,
    inst_rule1: &InstRule,
    inst_rule2: &InstRule,
    swap: bool,
  ) -> ir::MethodRel {
    let rule_id1 = format!(
      "{}.{}",
      self.module.name.to_string(),
      inst_rule1.to_string()
    );
    let rule_id2 = format!(
      "{}.{}",
      self.module.name.to_string(),
      inst_rule2.to_string()
    );
    if let Some(rel) =
      self.conflict_map.get(&(rule_id1.clone(), rule_id2.clone()))
    {
      log::debug!("conflict found by conflict map, {} vs. {} : {}", rule_id1, rule_id2, rel.to_string());
      return *rel;
    }

    if !string_match(&inst_rule1.path, &inst_rule2.path) {
      // log::debug!("no conflict found by path match");
      return ir::MethodRel::CF;
    }

    let module_name1 = self.resolve_path(&inst_rule1.path);
    let module_name2 = self.resolve_path(&inst_rule2.path);
    assert!(module_name1 == module_name2);
    let rel = if swap {
      self.rule_conflict(
        module_name1,
        inst_rule2.rule_name.clone(),
        inst_rule1.rule_name.clone(),
      )
    } else {
      self.rule_conflict(
        module_name1,
        inst_rule1.rule_name.clone(),
        inst_rule2.rule_name.clone(),
      )
    };
    if rel == ir::MethodRel::CF || rel == ir::MethodRel::C {
      self
        .conflict_map
        .insert((rule_id1.clone(), rule_id2.clone()), rel.clone());
      self
        .conflict_map
        .insert((rule_id2.clone(), rule_id1.clone()), rel.clone());
    } else if (rel == ir::MethodRel::SA && !swap)
      || (rel == ir::MethodRel::SB && swap)
    {
      self
        .conflict_map
        .insert((rule_id1.clone(), rule_id2.clone()), ir::MethodRel::SA);
      self
        .conflict_map
        .insert((rule_id2.clone(), rule_id1.clone()), ir::MethodRel::SB);
    } else {
      self
        .conflict_map
        .insert((rule_id1.clone(), rule_id2.clone()), ir::MethodRel::SB);
      self
        .conflict_map
        .insert((rule_id2.clone(), rule_id1.clone()), ir::MethodRel::SA);
    }
    return rel;
  }

  pub fn check_conflict(
    &mut self,
    inst_rule1: ir::InstRule,
    inst_rule2: ir::InstRule,
  ) -> bool {
    // use conflict_map here!
    let mut rule_id1 = format!(
      "{}.{}",
      self.module.name.to_string(),
      inst_rule1.to_string()
    );
    let mut rule_id2: String = format!(
      "{}.{}",
      self.module.name.to_string(),
      inst_rule2.to_string()
    );
    if let Some(rel) =
      self.conflict_map.get(&(rule_id1.clone(), rule_id2.clone()))
    {
      // log::debug!(
      //   "conflict found from conflict map, {} vs. {} : {}",
      //   rule_id1,
      //   rule_id2,
      //   rel.to_string()
      // );
      return matches!(rel, ir::MethodRel::C | ir::MethodRel::SB);
    }

    if inst_rule1.path.len() == inst_rule2.path.len() {

      let rel = self.conflict(&inst_rule1, &inst_rule2, false);
      if rel == ir::MethodRel::C || rel == ir::MethodRel::SB {
        return true;
      } else {
        return false;
      }
    } else {
      let mut final_rel = ir::MethodRel::CF;
      let mut inst_rule1 = inst_rule1.clone();
      let mut inst_rule2 = inst_rule2.clone();
      let mut path1 = inst_rule1.path.clone();
      let mut path2 = inst_rule2.path.clone();
      let mut swap = false;
      if path1.len() > path2.len() {
        std::mem::swap(&mut inst_rule1, &mut inst_rule2);
        std::mem::swap(&mut path1, &mut path2);
        swap = true;
      }
      let path_len_delta = path2.len() - path1.len();
      let mut inst1_subrules: Vec<InstRule> = vec![inst_rule1];
      for _ in 0..path_len_delta {
        let mut deeper_subrules = vec![];
        for inst_rule in inst1_subrules.into_iter() {
          deeper_subrules.extend(self.sub_inst_rules_from(&inst_rule));
        }
        inst1_subrules = deeper_subrules;
      }

      for inst_rule in inst1_subrules.into_iter() {
        let rel = self.conflict(&inst_rule, &inst_rule2, swap);
        match rel {
          ir::MethodRel::CF => {
            final_rel = final_rel;
          }
          ir::MethodRel::C => {
            final_rel = ir::MethodRel::C;
          }
          ir::MethodRel::SA => {
            if final_rel == ir::MethodRel::SA || final_rel == ir::MethodRel::CF
            {
              final_rel = ir::MethodRel::SA;
            } else {
              final_rel = ir::MethodRel::C;
            }
          }
          ir::MethodRel::SB => {
            if final_rel == ir::MethodRel::SB || final_rel == ir::MethodRel::CF
            {
              final_rel = ir::MethodRel::SB;
            } else {
              final_rel = ir::MethodRel::C;
            }
          }
        }
      }
      if final_rel == ir::MethodRel::CF || final_rel == ir::MethodRel::C {
        self
          .conflict_map
          .insert((rule_id1.clone(), rule_id2.clone()), final_rel.clone());
        self
          .conflict_map
          .insert((rule_id2.clone(), rule_id1.clone()), final_rel.clone());
      } else if final_rel == ir::MethodRel::SA {
        self
          .conflict_map
          .insert((rule_id1.clone(), rule_id2.clone()), ir::MethodRel::SA);
        self
          .conflict_map
          .insert((rule_id2.clone(), rule_id1.clone()), ir::MethodRel::SB);
      } else {
        self
          .conflict_map
          .insert((rule_id1.clone(), rule_id2.clone()), ir::MethodRel::SB);
        self
          .conflict_map
          .insert((rule_id2.clone(), rule_id1.clone()), ir::MethodRel::SA);
      }
      if final_rel == ir::MethodRel::C || final_rel == ir::MethodRel::SB {
        return true;
      } else {
        return false;
      }
    }
  }
}

pub fn invoke_op_at_step<'a>(
  op: &'a ir::Op,
  step: u32,
  data: &'a VisitorData,
  msched: &'a MSchedPass,
) -> Vec<InstRule> {
  match op.inner() {
    ir::OpEnum::Invoke(InvokeOp { inst_rule, .. }) => {
      let module_name = data.resolve_path(&inst_rule.path);
      let rule_path: RuleId = RuleId::from(&module_name, &inst_rule.rule_name);
      let rule_sched_info = msched.get_sched_info(&rule_path);
      assert!(
        rule_sched_info.stat.num_cycles.is_some(),
        "only static rule support per-step conflict checking"
      );
      if let Some(step) =
        rule_sched_info.hy_steps.get(&TimingExpr::const_(step))
      {
        step
          .ops
          .iter()
          .filter_map(|op| match &op.op.inner() {
            ir::OpEnum::Invoke(x) => {
              Some(x.inst_rule.clone().pre_extend(&inst_rule.path))
            }
            _ => None,
          })
          .collect()
      } else {
        vec![inst_rule.clone()]
      }
    }
    _ => unreachable!(),
  }
}

// check if x[x_step] and y[y_step] are in conflict
pub fn check_conflict_at_step(
  x: &ir::Op,
  x_step: u32,
  y: &ir::Op,
  y_step: u32,
  data: &mut VisitorData,
  msched: &MSchedPass,
) -> bool {
  let x_inst_rules = invoke_op_at_step(x, x_step, data, msched);
  let y_inst_rules = invoke_op_at_step(y, y_step, data, msched);

  for x_inst_rule in x_inst_rules.iter() {
    for y_inst_rule in y_inst_rules.iter() {
      if data.check_conflict(x_inst_rule.clone(), y_inst_rule.clone()) {
        return true;
      }
    }
  }
  false
}
