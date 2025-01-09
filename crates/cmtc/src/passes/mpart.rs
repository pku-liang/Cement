use std::vec;

use anyhow::bail;
use builtin::{fifo1_gen, reg_gen, wire_gen};
use indexmap::{IndexMap, IndexSet};
use petgraph::data;

use crate::{RuleTiming, TimingPartialOrder};

use super::*;

pub type RuleDeriveTable = IndexMap<RuleId, Vec<RuleId>>;
pub struct MPartPass {
  pub rule_sched_table: RuleSchedTable,
  pub value_context_table: ValueContextTable,
  pub rule_derive_table: RuleDeriveTable,
}

impl MPartPass {
  pub fn with(msched: MSchedPass) -> Self {
    Self {
      rule_sched_table: msched.rule_sched_table,
      value_context_table: msched.value_context_table,
      rule_derive_table: IndexMap::new(),
    }
  }

  pub fn stat_by_inst_rule(
    &self,
    inst_rule: &ir::InstRule,
    data: &mut VisitorData,
  ) -> RuleSchedStat {
    let module_name = data.resolve_path(inst_rule.path.as_slice());
    let rule_id = RuleId::from(&module_name, &inst_rule.rule_name);
    self.rule_sched_table.get(&rule_id).unwrap().stat.clone()
  }
}

impl Visitor for MPartPass {
  fn name() -> &'static str {
    "mpart"
  }

  /// mpart is not applicable to tb module
  fn skip_tb() -> bool {
    true
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    let rule_id = RuleId::from(&data.module.name(), &data.rule().name);
    let RuleSched {
      stat,
      timing_po: _,
      exit_cycle: _,
      ..
    } = self.rule_sched_table.get(&rule_id).unwrap().clone();

    if stat.is_combinational {
      // for comb/single-cycle rules, we don't need to partition
      let mut rules = vec![data.take_rule()];
      Ok((rules, vec![]))
    } else {
      bail!("multi-cycle rule is not supported");
    }
  }

  fn after_visit_modules(
    &mut self,
    circuit: &mut ir::Circuit,
  ) -> anyhow::Result<()> {
    // delete rules that have been partitioned
    circuit.modules.iter_mut().for_each(|module| {
      let module_name = module.name().to_string();
      module.rules_mut().retain(|rule| {
        !self
          .rule_derive_table
          .contains_key(&RuleId::from(&module_name, rule.name()))
      });
    });
    Ok(())
  }
}
