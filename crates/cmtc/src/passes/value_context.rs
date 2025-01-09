use indexmap::IndexMap;

use super::*;

pub struct ValueContext {
  pub texpr_to_defs: IndexMap<ir::TimingExpr, Vec<ir::ValueId>>,
  pub texpr_to_uses: IndexMap<ir::TimingExpr, Vec<ir::ValueId>>,
  pub ret_values_at_texpr: IndexMap<ir::TimingExpr, Vec<ir::ValueId>>,
  // single-assigment
  pub def_texpr: SecondaryMap<ir::ValueId, ir::TimingExpr>,
  pub use_texpr: SecondaryMap<ir::ValueId, Vec<ir::TimingExpr>>,
}

impl<'a> VisitorData<'a> {
  pub fn get_output_delta(
    &self,
    op: &ir::Op,
    idx: usize,
    rule_sched_infos: &RuleSchedTable,
    value_context_table: &ValueContextTable,
  ) -> Option<u32> {
    match op.inner() {
      ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) => {
        let rule_id = self.rule_id_by_inst_rule(inst_rule);
        let rule_sched = rule_sched_infos.get(&rule_id)?;
        if rule_sched.stat.guard_always_true
          && rule_sched.stat.num_cycles.is_some()
        {
          let value_context = value_context_table.get(&rule_id)?;
          let output_def_cycle = value_context
            .texpr_by_res_idx(self, Some(rule_id.clone()), idx)
            .unwrap();
          Some(output_def_cycle.as_u32().unwrap())
        } else {
          None
        }
      }
      _ => None,
    }
  }

  pub fn get_input_delta(
    &self,
    op: &ir::Op,
    idx: usize,
    rule_sched_infos: &RuleSchedTable,
    value_context_table: &ValueContextTable,
  ) -> Option<u32> {
    match op.inner() {
      ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) => {
        let rule_id = self.rule_id_by_inst_rule(inst_rule);
        let rule_sched = rule_sched_infos.get(&rule_id)?;
        if rule_sched.stat.guard_always_true
          && rule_sched.stat.num_cycles.is_some()
        {
          let value_context = value_context_table.get(&rule_id)?;
          let input_use_cycle = value_context
            .texpr_by_arg_idx(self, Some(rule_id.clone()), idx)
            .unwrap();
          Some(input_use_cycle.as_u32().unwrap())
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

impl ValueContext {
  pub fn build(
    hy_steps: &IndexMap<ir::TimingExpr, OpsPerStep>,
    end_cycles: &AutoVec<ir::TimingExpr>,
    rule_sched: &RuleSchedTable,
    value_context_table: &ValueContextTable,
    is_dynamic: bool,
    exit_cycle: ir::TimingExpr,
    data: &mut VisitorData,
  ) -> Result<Self, anyhow::Error> {
    let mut texpr_to_defs = IndexMap::new();
    let mut texpr_to_uses = IndexMap::new();
    let mut def_texpr = SecondaryMap::new();
    let mut use_texpr = SecondaryMap::new();

    // extract def/use texprs from hy_steps
    for (texpr, ops) in hy_steps.iter() {
      for op in ops.ops.iter() {
        for (output_idx, def) in op.op.outputs().enumerate() {
          let texpr = if let Some(delta) = data.get_output_delta(
            &op.op,
            output_idx,
            rule_sched,
            value_context_table,
          ) {
            texpr.clone().add_const(delta)
          } else {
            end_cycles.get(op.idx.unwrap()).clone()
          };
          if def_texpr.contains_key(def) {
            // op must be a return op, where SSA is violated
            assert!(op.op.is_return(), "only return op can violate SSA");
          } else {
            def_texpr.insert(def, texpr.clone());
          }
          texpr_to_defs
            .entry(texpr.clone())
            .or_insert(vec![])
            .push(def);
        }
        for (input_idx, use_) in op.op.inputs().enumerate() {
          let texpr = if let Some(delta) = data.get_input_delta(
            &op.op,
            input_idx,
            rule_sched,
            value_context_table,
          ) {
            texpr.clone().add_const(delta)
          } else {
            texpr.clone()
          };
          use_texpr
            .entry(use_)
            .unwrap()
            .or_insert(vec![])
            .push(texpr.clone());
          texpr_to_uses
            .entry(texpr.clone())
            .or_insert(vec![])
            .push(use_);
        }
      }
    }

    for arg in data.rule().inputs() {
      let first_use_texpr = if !is_dynamic {
        // static: args are defed at the first step when they are used
        use_texpr
          .get(arg)
          .cloned()
          .unwrap_or(vec![ir::TimingExpr::const_(0)])
          .first()
          .cloned()
          .ok_or(data.report_error(format!(
            "cannot find first use texpr for arg {}",
            data.print_value(arg)
          )))?
      } else {
        // dynamic: args must arrive at the first step
        ir::TimingExpr::const_(0)
      };
      texpr_to_defs
        .entry(first_use_texpr.clone())
        .or_insert(vec![])
        .push(arg);
      def_texpr.insert(arg, first_use_texpr);
    }

    // return values are the last op's outputs
    let ret_values = data.rule().ops.last().map(|op| op.outputs());
    let mut ret_values_at_texpr = IndexMap::new();
    if let Some(ret_values) = ret_values {
      // couple ret_values and rule outputs
      for (ret_value, output) in ret_values.zip(data.rule().outputs()) {
        let texpr = if !is_dynamic {
          // static: the output is defed at the step when ret_value is defed,
          // also the step when ret_value is used
          def_texpr
            .get(ret_value)
            .cloned()
            .unwrap_or(ir::TimingExpr::const_(0))
            .clone()
        } else {
          // dynamic: the output is defed at the exit cycle
          exit_cycle.clone()
        };
        texpr_to_defs
          .entry(texpr.clone())
          .or_insert(vec![])
          .push(output);
        def_texpr.insert(output, texpr.clone());
        texpr_to_uses
          .entry(texpr.clone())
          .or_insert(vec![])
          .push(ret_value);
        use_texpr
          .entry(ret_value)
          .unwrap()
          .or_insert(vec![])
          .push(texpr.clone());
        ret_values_at_texpr
          .entry(texpr.clone())
          .or_insert(vec![])
          .push(ret_value);
      }
    } else {
      // ext module's rule
      for output in data.rule().outputs() {
        texpr_to_defs
          .entry(ir::TimingExpr::const_(0))
          .or_insert(vec![])
          .push(output);
        def_texpr.insert(output, ir::TimingExpr::const_(0));
      }
    }

    Ok(Self {
      texpr_to_defs,
      texpr_to_uses,
      ret_values_at_texpr,
      def_texpr,
      use_texpr,
    })
  }

  pub fn dump(&self) -> String {
    let mut s = String::new();
    s.push_str("\nValue Context:\n");
    for (value_id, texpr) in self.def_texpr.iter() {
      s.push_str(&format!("\t{} defed at {}\n", value_id, texpr.to_string()));
      s.push_str(&format!(
        "\t\tuses: {:?}\n",
        self.use_texpr.get(value_id).map(|texprs| texprs
          .iter()
          .map(|texpr| texpr.to_string())
          .collect::<Vec<_>>())
      ));
    }
    s
  }

  pub fn replace_ret_values(
    &mut self,
    value_id: ir::ValueId,
    new_value_id: ir::ValueId,
    texpr: ir::TimingExpr,
  ) -> anyhow::Result<()> {
    self
      .ret_values_at_texpr
      .get_mut(&texpr)
      .into_iter()
      .for_each(|ret_values| {
        for ret_value in ret_values {
          if *ret_value == value_id {
            *ret_value = new_value_id;
          }
        }
      });
    Ok(())
  }
  pub fn args_at_texpr(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
    texpr: &ir::TimingExpr,
  ) -> Result<Vec<ir::ValueId>, anyhow::Error> {
    Ok(
      data
        .rule_args(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .filter(|&&arg| self.def_texpr.get(arg).unwrap().eq(texpr))
        .cloned()
        .collect(),
    )
  }

  pub fn res_at_texpr(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
    texpr: &ir::TimingExpr,
  ) -> Result<Vec<ir::ValueId>, anyhow::Error> {
    Ok(
      data
        .rule_res(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .filter(|&&res| self.def_texpr.get(res).unwrap().eq(texpr))
        .cloned()
        .collect(),
    )
  }

  pub fn ret_values_at_texpr(
    &self,
    texpr: &ir::TimingExpr,
  ) -> Result<Vec<ir::ValueId>, anyhow::Error> {
    Ok(
      self
        .ret_values_at_texpr
        .get(texpr)
        .cloned()
        .unwrap_or_default(),
    )
  }

  pub fn args(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
  ) -> Result<Vec<(ir::TimingExpr, ir::ValueId)>, String> {
    Ok(
      data
        .rule_args(rule_id.as_ref())
        .ok_or(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        ))?
        .iter()
        .map(|&arg| (self.def_texpr.get(arg).unwrap().clone(), arg))
        .collect(),
    )
  }

  pub fn res(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
  ) -> Result<Vec<(ir::TimingExpr, ir::ValueId)>, String> {
    Ok(
      data
        .rule_res(rule_id.as_ref())
        .ok_or(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        ))?
        .iter()
        .map(|&res| (self.def_texpr.get(res).unwrap().clone(), res))
        .collect(),
    )
  }

  pub fn args_idx_at_texpr(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
    texpr: &ir::TimingExpr,
  ) -> anyhow::Result<Vec<usize>> {
    Ok(
      data
        .rule_args(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .enumerate()
        .filter(|&(_, &arg)| self.def_texpr.get(arg).unwrap().eq(texpr))
        .map(|(idx, _)| idx)
        .collect(),
    )
  }

  pub fn res_idx_at_texpr(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
    texpr: &ir::TimingExpr,
  ) -> anyhow::Result<Vec<usize>> {
    Ok(
      data
        .rule_res(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .enumerate()
        .filter(|&(_, &res)| self.def_texpr.get(res).unwrap().eq(texpr))
        .map(|(idx, _)| idx)
        .collect(),
    )
  }

  pub fn args_idx(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
  ) -> anyhow::Result<Vec<(String, usize, ir::TimingExpr)>> {
    Ok(
      data
        .rule_args(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .enumerate()
        .map(|(idx, &arg)| {
          (
            format!(
              "_feed_arg_{}",
              data.print_value_in_module(
                arg,
                if let Some(rule_id) = &rule_id {
                  data.circuit.module(&rule_id.module_name).unwrap()
                } else {
                  &data.module
                }
              )
            ),
            idx,
            self.def_texpr.get(arg).unwrap().clone(),
          )
        })
        .collect(),
    )
  }

  pub fn res_idx(
    &self,
    data: &mut VisitorData,
    rule_id: Option<RuleId>,
  ) -> anyhow::Result<Vec<(String, usize, ir::TimingExpr)>> {
    Ok(
      data
        .rule_res(rule_id.as_ref())
        .ok_or(data.report_error(format!(
          "cannot find rule {}",
          rule_id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or(data.rule().name().to_string())
        )))?
        .iter()
        .enumerate()
        .map(|(idx, &res)| {
          (
            format!(
              "_get_res_{}",
              data.print_value_in_module(
                res,
                if let Some(rule_id) = &rule_id {
                  data.circuit.module(&rule_id.module_name).unwrap()
                } else {
                  &data.module
                }
              )
            ),
            idx,
            self.def_texpr.get(res).unwrap().clone(),
          )
        })
        .collect(),
    )
  }

  pub fn texpr_by_res_idx(
    &self,
    data: &VisitorData,
    rule_id: Option<RuleId>,
    idx: usize,
  ) -> anyhow::Result<ir::TimingExpr> {
    data
      .rule_res(rule_id.as_ref())
      .ok_or(data.report_error(format!(
        "cannot find rule {}",
        rule_id
          .map(|id| id.to_string())
          .unwrap_or(data.rule().name().to_string())
      )))?
      .get(idx)
      .map(|value_id| self.def_texpr.get(*value_id).unwrap().clone())
      .ok_or(
        data
          .report_error(format!("cannot find res idx {}", idx))
          .into(),
      )
  }

  pub fn texpr_by_arg_idx(
    &self,
    data: &VisitorData,
    rule_id: Option<RuleId>,
    idx: usize,
  ) -> anyhow::Result<ir::TimingExpr> {
    data
      .rule_args(rule_id.as_ref())
      .ok_or(data.report_error(format!(
        "cannot find rule {}",
        rule_id
          .map(|id| id.to_string())
          .unwrap_or(data.rule().name().to_string())
      )))?
      .get(idx)
      .map(|value_id| self.def_texpr.get(*value_id).unwrap().clone())
      .ok_or(
        data
          .report_error(format!("cannot find arg idx {}", idx))
          .into(),
      )
  }
}
