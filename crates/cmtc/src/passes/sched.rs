use super::*;

use std::cmp::{Ordering, Reverse};

use indexmap::IndexSet;
use log::info;
use petgraph::data;

use crate::Cmp;

/// Schedule pass:
/// determine the total schedule order of all rules
pub struct SchedPass {
  pub modules_to_synthesize: IndexSet<String>,
}

impl SchedPass {
  pub fn with(modules_to_synthesize: IndexSet<String>) -> Self {
    Self {
      modules_to_synthesize,
    }
  }
}

impl Visitor for SchedPass {
  fn name() -> &'static str {
    "mod-sched"
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    Ok((vec![data.rule().clone()], vec![]))
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    // schedule all the rules contained in the module and its instances

    let mut all_rules = vec![];

    for instance in data.module.instances() {
      let module = data
        .circuit
        .module(&instance.module)
        .ok_or(data.report_error(format!(
          "module {} not found in circuit",
          instance.module
        )))?;
      if self.modules_to_synthesize.contains(&instance.module)
        || module.is_virtual()
      {
        // skip instances of a module to be synthesized
        continue;
      }
      let mut schedule_in_instance = module.schedules();
      all_rules.extend(
        schedule_in_instance
          .next()
          .ok_or(data.report_error(format!(
            "expect only one schedule in module {}, but got none",
            instance.module
          )))?
          .into_iter()
          .filter_map(|inst_rule| {
            // only keep always rule
            if let Some(rule) = module.rule_by_name(&inst_rule.rule_name) {
              if rule.is_always() {
                Some(inst_rule.pre_extend(&vec![instance.name.clone()]))
              } else {
                None
              }
            } else {
              Some(inst_rule.pre_extend(&vec![instance.name.clone()]))
            }
          })
          .collect::<Vec<_>>(),
      );
      if schedule_in_instance.next().is_some() {
        return Err(data.report_error(format!(
          "expect only one schedule in module {}, but got more than one",
          instance.module
        )))?;
      }
    }
    all_rules.extend(
      data
        .module
        .rules()
        .filter_map(|r| {
          if r.is_always() || !r.is_private() {
            Some(
              InstRule {
                path: vec![],
                rule_name: r.name.clone(),
              }
              .canonicalize(),
            )
          } else {
            None
          }
        })
        .collect::<Vec<_>>(),
    );

    log::debug!("all rules for module {}: {:?}", data.module.name, all_rules);

    self.schedule_rules(all_rules, data)?;

    log::debug!(
      "schedule: {:?}",
      data
        .module
        .schedules()
        .next()
        .ok_or(data.report_error(format!(
          "expect at least one schedule in module {}",
          data.module.name
        )))?
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
    );
    Ok(())
  }
}

impl SchedPass {
  fn schedule_rules(
    &mut self,
    all_rules: Vec<InstRule>,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let rules_with_idx = data.schedule_rules(all_rules)?;

    data.module.clear_rule_sched_orders();

    // TODO: add optimization for rule scheduling
    // 1. use method-rels(SA/SB) to schedule rules
    // 2. adjust schedule to avoid conflicts

    data.module.add_schedule(
      rules_with_idx
        .into_iter()
        .map(|r| r.canonicalize())
        .collect(),
    );
    Ok(())
  }
}
