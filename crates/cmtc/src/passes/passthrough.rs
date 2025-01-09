use super::*;
use itertools::{chain, Itertools};

pub struct PassThrough;

impl Visitor for PassThrough {
  fn name() -> &'static str {
    "PassThrough"
  }

  fn visit_op_impl(
    &mut self,
    op: &mut ir::Op,
    _data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    if let ir::OpEnum::Invoke(invoke) = op.inner_mut() {
      if invoke.inst_rule.path.len() > 1 {
        let path: Vec<String> = invoke.inst_rule.path.drain(..).collect();
        let mut path = path.into_iter();
        invoke.inst_rule.path = vec![path.next().unwrap()];
        invoke.inst_rule.rule_name =
          chain!(path, [invoke.inst_rule.rule_name.clone()]).join("_");
        // log::info!("{:?}", invoke.inst_rule);
      }
    }
    Ok(())
  }
}

impl PassThrough {
  pub fn new() -> Self {
    Self {}
  }
}
