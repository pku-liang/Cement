use std::collections::HashSet;

use super::*;

/// MethodRelInferPass
/// NOTE: must be run after SetPrivatePass
/// should get methodrels including SA and SB from schedules
/// for example, schedule [method m0, method m1] indicates m0 SA m1
/// also, from the opposite direction, SA/SB methodrels should also indicate the
/// schedule for example, m0 SA m1 indicates schedule [m0, m1]
pub struct MethodRelInferPass {}

impl MethodRelInferPass {
  pub fn new() -> Self {
    Self {}
  }
}

impl Visitor for MethodRelInferPass {
  fn name() -> &'static str {
    "methodrel_infer"
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    // first, use methodrels to indicate schedule
    let mut new_schedules = vec![];
    for methodrel in data.module.method_rels() {
      match methodrel {
        ir::RuleRel::Method {
          rel: ir::MethodRel::SA,
          lhs: m0s,
          rhs: m1s,
        } => {
          for m0 in m0s {
            for m1 in m1s {
              new_schedules.push(vec![m0.clone(), m1.clone()]);
            }
          }
        }
        ir::RuleRel::Method {
          rel: ir::MethodRel::SB,
          lhs: m0s,
          rhs: m1s,
        } => {
          for m0 in m0s {
            for m1 in m1s {
              new_schedules.push(vec![m1.clone(), m0.clone()]);
            }
          }
        }
        _ => {}
      }
    }

    for schedule in new_schedules {
      data.module.add_schedule(schedule);
    }

    // next extract exisiting public methods, and use the schedules to sort them
    let public_methods = data
      .module
      .rules()
      .filter_map(|rule| {
        if !rule.is_private() && rule.is_method() {
          Some(ir::InstRule::new(vec![rule.name.clone()]).canonicalize())
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    let mut public_method_set =
      public_methods.clone().into_iter().collect::<HashSet<_>>();

    let public_methods = data.schedule_rules(public_methods)?;

    // remove all method-only schedules
    data.module.rule_rels.retain(|rule_rel| match rule_rel {
      &ir::RuleRel::Schedule(ref schedule) => {
        schedule.iter().any(|inst_rule| {
          !public_method_set.contains(&inst_rule.canonicalize())
        })
      }
      _ => true,
    });

    // add a new schedule due to public_methods_with_idx
    data.module.add_schedule(public_methods.clone());

    // add new rule_x C rule_x for all rule_x with any inputs
    let mut rules_with_inputs = vec![];
    for rule in data.module.rules() {
      if rule.inputs().len() > 0 {
        rules_with_inputs.push(rule.name.clone());
      }
    }

    for rule_name in rules_with_inputs {
      data.module.add_rule_rel(ir::RuleRel::Method {
        rel: ir::MethodRel::C,
        lhs: vec![InstRule::new(vec![rule_name.clone()]).canonicalize()],
        rhs: vec![InstRule::new(vec![rule_name.clone()]).canonicalize()],
      });
    }

    log::debug!(
      "after methodrel infer:\n\t{}",
      data
        .module
        .rule_rels()
        .map(|r| r.ir_dump())
        .collect::<Vec<_>>()
        .join("\n\t")
    );
    Ok(())
  }
}
