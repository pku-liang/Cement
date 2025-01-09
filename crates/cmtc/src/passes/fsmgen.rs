use super::*;
use crate::{
  BodyOp, BranchOp, Cmp, ForOp, Module, Op, ParOp, Prim, Rule, TypedLit,
};

pub mod pass;
pub use pass::*;

mod templates;

impl Visitor for FSMGenPass {
  fn name() -> &'static str {
    "FSMGen"
  }

  /// tb module should have no fsm
  fn skip_tb() -> bool {
    true
  }

  fn before_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    self.cur_module_name = data.module.name.clone();
    self.step_cnt = 0;
    self.seq_cnt = 0;
    self.par_cnt = 0;
    self.branch_cnt = 0;
    self.for_cnt = 0;
    self.pipeline_cnt = 0;
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    let rule = data.take_rule();
    if matches!(rule.timing, ir::RuleTiming::FSM) {
      assert!(rule.ops.len() == 1, "FSM rule can only have one op!");
      let fsm = self.visit_op(rule.ops.first().unwrap());
      let rules = self.gen_rules(fsm, data, rule);
      self.fsm_cnt += 1;
      Ok((rules, vec![]))
    } else if matches!(rule.timing, ir::RuleTiming::Pipeline) {
      let fsm = self.visit_pipeline(&rule.ops);
      let rules = self.gen_rules(fsm, data, rule);
      self.fsm_cnt += 1;
      Ok((rules, vec![]))
    } else {
      Ok((vec![rule], vec![]))
    }
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    for (i, s) in self.top_fsms.drain(..).enumerate() {
      data.module.add_instance(format!("GEN_fsm_{i}"), s);
    }
    for (lhs, rhs) in self.cfs.drain(..) {
      data.module.add_method_rel(cmtrs::MethodRel::CF, lhs, rhs);
    }
    Ok(())
  }

  fn after_pass(&mut self, circuit: &mut crate::Circuit) -> anyhow::Result<()> {
    for module in self.gen_modules.drain(..) {
      circuit.add_module(module);
    }
    Ok(())
  }
}
