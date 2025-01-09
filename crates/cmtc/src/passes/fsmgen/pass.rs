use super::templates::*;
use super::*;

use cmtir::RadixIntLit;
use cmtrs::stl::GeneratedModule;
use cmtrs::Instance;
use indexmap::IndexSet;
use itertools::{chain, izip, Itertools};

pub struct FSMGenPass {
  pub fsm_cnt: usize,
  pub step_cnt: usize,
  pub seq_cnt: usize,
  pub par_cnt: usize,
  pub branch_cnt: usize,
  pub for_cnt: usize,
  pub pipeline_cnt: usize,
  pub cur_module_name: String,
  pub cfs: Vec<(Vec<InstRule>, Vec<InstRule>)>,
  pub gen_modules: Vec<ir::Module>,
  pub top_fsms: Vec<String>,
}

impl FSMGenPass {
  pub fn new() -> Self {
    FSMGenPass {
      fsm_cnt: 0,
      step_cnt: 0,
      seq_cnt: 0,
      par_cnt: 0,
      branch_cnt: 0,
      for_cnt: 0,
      pipeline_cnt: 0,
      cur_module_name: "".to_string(),
      cfs: Vec::new(),
      gen_modules: Vec::new(),
      top_fsms: Vec::new(),
    }
  }

  pub fn visit_step(&mut self, step: &BodyOp) -> FSMRet {
    let step_name = format!("step{}", self.step_cnt);
    let module_name = self.step_name();

    self.step_cnt += 1;

    FSMRet {
      module: Some(make_step(module_name, step_name.clone())),
      in_rules: Vec::new(),
      out_rules: vec![Rule::always(
        step_name,
        vec![],
        vec![],
        vec![],
        step.ops.clone(),
        ir::RuleTiming::SingleCycle,
      )],
    }
  }

  pub fn visit_seq(&mut self, seq: &BodyOp) -> FSMRet {
    let module_name = self.seq_name();
    self.seq_cnt += 1;

    let n = seq.ops.len();
    assert!(n != 0);
    let mut children: Vec<FSMRet> =
      seq.ops.iter().map(|op| self.visit_op(op)).collect();

    if n == 1 {
      return children.into_iter().next().unwrap();
    }

    for i in 0..n {
      for j in (i + 1)..n {
        self.cfs.push((
          children[i].out_rules_2_insts(),
          children[j].out_rules_2_insts(),
        ));
      }
    }

    let (mut ret, children) = merge_children(children);
    ret.module = Some(make_seq(module_name, children));

    ret
  }

  pub fn visit_par(&mut self, par: &ParOp) -> FSMRet {
    let module_name = self.par_name();
    self.par_cnt += 1;

    let n = par.ops.len();
    assert!(n != 0);
    let mut children: Vec<FSMRet> =
      par.ops.iter().map(|op| self.visit_op(op)).collect();

    if n == 1 {
      return children.into_iter().next().unwrap();
    }

    let (mut ret, children) = merge_children(children);
    ret.module = Some(make_par(module_name, children));
    ret
  }

  pub fn visit_branch(&mut self, branch: &BranchOp) -> FSMRet {
    let module_name = self.branch_name();
    self.branch_cnt += 1;

    let mut rets = Vec::new();
    rets.push(self.visit_op(&branch.then_body));
    if let Some(else_body) = branch.else_body.as_ref() {
      rets.push(self.visit_op(else_body));
      self
        .cfs
        .push((rets[0].out_rules_2_insts(), rets[1].out_rules_2_insts()));
    }

    let (mut ret, mut children) = merge_children(rets);

    assert!(branch.cond.num_outputs() == 1);
    let cond_output = branch.cond.output(0);
    let cond_name = format!("{}_cond", module_name);
    ret.new_in_rule(Rule::always(
      cond_name.clone(),
      vec![],
      vec![],
      vec![],
      vec![
        branch.cond.clone(),
        Op::invoke(
          InstRule::from_str(&format!(
            "{}.{}_cond",
            self.fsm_name(),
            module_name
          )),
          vec![cond_output],
          vec![],
        ),
      ],
      ir::RuleTiming::SingleCycle,
    ));

    let mut children = children.into_iter();
    let then_body = children.next().unwrap();
    let else_body = children.next();

    ret.module = Some(make_branch(
      module_name,
      cond_name,
      then_body,
      else_body,
      branch.loose,
    ));

    ret
  }

  pub fn visit_for(&mut self, for_op: &ForOp) -> FSMRet {
    let module_name = self.for_name();
    self.for_cnt += 1;

    let body = self.visit_op(&for_op.body);

    let (mut ret, mut children) = merge_children(vec![body]);
    let body = children.into_iter().next().unwrap();

    if for_op.loose {
      assert!(for_op.cond.num_outputs() == 1);
      let cond_output = for_op.cond.output(0);
      let cond_name = format!("{}_cond", module_name);
      ret.new_in_rule(Rule::always(
        cond_name.clone(),
        vec![],
        vec![],
        vec![],
        vec![
          for_op.cond.clone(),
          Op::invoke(
            InstRule::from_str(&format!(
              "{}.{}_cond",
              self.fsm_name(),
              module_name
            )),
            vec![cond_output],
            vec![],
          ),
        ],
        ir::RuleTiming::SingleCycle,
      ));

      let init_name = if let Some(init) = &for_op.init {
        let init_name = format!("{}_init", module_name);
        ret.new_out_rule(Rule::always(
          init_name.clone(),
          vec![],
          vec![],
          vec![],
          vec![init.clone()],
          ir::RuleTiming::SingleCycle,
        ));
        Some(init_name)
      } else {
        None
      };

      let update_name = format!("{}_update", module_name);
      ret.new_out_rule(Rule::always(
        update_name.clone(),
        vec![],
        vec![],
        vec![],
        vec![for_op.update.clone()],
        ir::RuleTiming::SingleCycle,
      ));

      if let Some(init_name) = &init_name {
        self.cfs.push((
          vec![InstRule::from_str(init_name)],
          vec![InstRule::from_str(&update_name)],
        ));
      }
      ret.module = Some(make_loose_for(
        module_name,
        init_name,
        cond_name,
        update_name,
        body,
      ));
    } else {
      let init_cond = for_op.init_cond.clone().unwrap();
      assert!(init_cond.num_outputs() == 1);
      let init_cond_output = init_cond.output(0);
      let init_cond_name = format!("{}_init_cond", module_name);
      ret.new_in_rule(Rule::always(
        init_cond_name.clone(),
        vec![],
        vec![],
        vec![],
        vec![
          init_cond,
          Op::invoke(
            InstRule::from_str(&format!(
              "{}.{}_init_cond",
              self.fsm_name(),
              module_name
            )),
            vec![init_cond_output],
            vec![],
          ),
        ],
        ir::RuleTiming::SingleCycle,
      ));

      assert!(for_op.cond.num_outputs() == 1);
      let update_cond_output = for_op.cond.output(0);
      let update_cond_name = format!("{}_update_cond", module_name);
      ret.new_in_rule(Rule::always(
        update_cond_name.clone(),
        vec![],
        vec![],
        vec![],
        vec![
          for_op.cond.clone(),
          Op::invoke(
            InstRule::from_str(&format!(
              "{}.{}_update_cond",
              self.fsm_name(),
              module_name
            )),
            vec![update_cond_output],
            vec![],
          ),
        ],
        ir::RuleTiming::SingleCycle,
      ));

      let init_name = if let Some(init) = &for_op.init {
        let init_name = format!("{}_init", module_name);
        ret.new_out_rule(Rule::always(
          init_name.clone(),
          vec![],
          vec![],
          vec![],
          vec![init.clone()],
          ir::RuleTiming::SingleCycle,
        ));
        Some(init_name)
      } else {
        None
      };

      let update_name = format!("{}_update", module_name);
      ret.new_out_rule(Rule::always(
        update_name.clone(),
        vec![],
        vec![],
        vec![],
        vec![for_op.update.clone()],
        ir::RuleTiming::SingleCycle,
      ));

      if let Some(init_name) = &init_name {
        self.cfs.push((
          vec![InstRule::from_str(init_name)],
          vec![InstRule::from_str(&update_name)],
        ));
      }
      ret.module = Some(make_tight_for(
        module_name,
        init_name,
        init_cond_name,
        update_name,
        update_cond_name,
        body,
      ));
    }

    ret
  }

  pub fn visit_pipeline(&mut self, ops: &Vec<ir::Op>) -> FSMRet {
    let module_name = self.pipeline_name();
    self.pipeline_cnt += 1;

    let n = ops.len();
    assert!(n != 0);
    let mut rets: Vec<FSMRet> =
      ops.iter().map(|op| self.visit_op(op)).collect();
    if n == 1 {
      return rets.into_iter().next().unwrap();
    }

    let rets = rets.into_iter().rev().collect();

    let (mut ret, children) = merge_children(rets);
    let children = children.into_iter().rev().collect();

    ret.module = Some(make_static_pipeline(module_name, children));

    ret
  }

  pub fn visit_op(&mut self, op: &ir::Op) -> FSMRet {
    match op.inner() {
      ir::OpEnum::Step(step) => self.visit_step(step),
      ir::OpEnum::Seq(seq) => self.visit_seq(seq),
      ir::OpEnum::Par(par) => self.visit_par(par),
      ir::OpEnum::Branch(branch) => self.visit_branch(branch),
      ir::OpEnum::For(for_op) => self.visit_for(for_op),
      _ => panic!("Invalid op type {:?} in a FSM/Pipeline", op),
    }
  }

  pub fn gen_rules(
    &mut self,
    fsm: FSMRet,
    data: &mut VisitorData,
    mut fsm_rule: Rule,
  ) -> Vec<Rule> {
    let FSMRet {
      module,
      in_rules,
      out_rules,
    } = fsm;
    let mut module = module.unwrap();
    self.top_fsms.push(module.get_name().to_string());
    let fsm_name = self.fsm_name();

    // guard
    let finish = data.module.add_value(None, Some(cmtir::Type::UInt(1)));
    let invoke = Op::invoke(
      InstRule::from_str(&format!("{}.finish", fsm_name)),
      vec![],
      vec![finish],
    );
    if fsm_rule.guard_ops.len() == 0 {
      fsm_rule.guard_ops = vec![invoke];
    } else {
      let old_guard = Op::block(fsm_rule.guard_ops);
      assert!(old_guard.num_outputs() == 1);
      let rst = data.module.add_value(None, Some(cmtir::Type::UInt(1)));
      let and = Op::binary_op(Prim::And, old_guard.output(0), finish, rst);
      fsm_rule.guard_ops = vec![invoke, old_guard, and];
    }

    // ops
    let invoke = Op::invoke(
      InstRule::from_str(&format!("{}.start", fsm_name)),
      vec![],
      vec![],
    );
    fsm_rule.ops = vec![invoke];

    // timing
    fsm_rule.timing = ir::RuleTiming::SingleCycle;

    let mut result = vec![];

    let rule_inputs: IndexSet<ir::ValueId> =
      fsm_rule.inputs().clone().into_iter().collect();
    let rule_outputs: IndexSet<ir::ValueId> =
      fsm_rule.outputs().clone().into_iter().collect();
    // let rule_outputs = fsm_rule.outputs().clone();

    // inputs
    for mut rule in in_rules {
      let mut inputs = IndexSet::new();
      let mut outputs = IndexSet::new();
      for op in chain!(&rule.guard_ops, &rule.ops) {
        check_ios(op, &rule_inputs, &rule_outputs, &mut inputs, &mut outputs);
      }
      rule.inputs_mut().extend(inputs.into_iter());
      rule.outputs_mut().extend(outputs.into_iter());
      result.push(rule);
    }

    result.push(fsm_rule);

    // outputs
    for mut rule in out_rules {
      let mut inputs = IndexSet::new();
      let mut outputs = IndexSet::new();
      for op in chain!(&rule.guard_ops, &rule.ops) {
        check_ios(op, &rule_inputs, &rule_outputs, &mut inputs, &mut outputs);
      }
      rule.inputs_mut().extend(inputs.into_iter());
      rule.outputs_mut().extend(outputs.into_iter());

      let rst = data.module.add_value(None, Some(cmtir::Type::UInt(1)));
      let invoke = Op::invoke(
        InstRule::from_str(&format!("{}.{}", fsm_name, rule.name)),
        vec![],
        vec![rst],
      );
      rule.guard_ops = vec![invoke];
      result.push(rule);
    }

    self.gen_modules.extend(module.make_modules());

    result
  }

  fn fsm_name(&self) -> String {
    format!("GEN_fsm_{}", self.fsm_cnt)
  }
  fn step_name(&self) -> String {
    format!("{}_step_{}", self.cur_module_name, self.step_cnt)
  }
  fn seq_name(&self) -> String {
    format!("{}_seq_{}", self.cur_module_name, self.seq_cnt)
  }
  fn par_name(&self) -> String {
    format!("{}_par_{}", self.cur_module_name, self.par_cnt)
  }
  fn branch_name(&self) -> String {
    format!("{}_branch_{}", self.cur_module_name, self.branch_cnt)
  }
  fn for_name(&self) -> String {
    format!("{}_for_{}", self.cur_module_name, self.for_cnt)
  }
  fn pipeline_name(&self) -> String {
    format!("{}_pipeline_{}", self.cur_module_name, self.pipeline_cnt)
  }
}

pub struct FSMChild {
  pub module: FSMModule,
  pub in_rules: Vec<String>,
  pub out_rules: Vec<String>,
}

pub struct FSMRet {
  pub module: Option<FSMModule>,
  pub in_rules: Vec<ir::Rule>,
  pub out_rules: Vec<ir::Rule>,
}

impl FSMRet {
  pub fn new_in_rule(&mut self, rule: ir::Rule) {
    self.in_rules.push(rule);
  }
  pub fn new_out_rule(&mut self, rule: ir::Rule) {
    self.out_rules.push(rule);
  }

  pub fn out_rules_2_insts(&self) -> Vec<InstRule> {
    self
      .out_rules
      .iter()
      .map(|o| InstRule::from_str(o.name()))
      .collect()
  }
}

fn merge_children(rets: Vec<FSMRet>) -> (FSMRet, Vec<FSMChild>) {
  let mut in_rules = Vec::new();
  let mut out_rules = Vec::new();
  let mut children: Vec<FSMChild> = Vec::new();

  for c in rets {
    children.push(FSMChild {
      module: c.module.unwrap(),
      in_rules: c.in_rules.iter().map(|r| r.name().to_string()).collect(),
      out_rules: c.out_rules.iter().map(|r| r.name().to_string()).collect(),
    });
    in_rules.extend(c.in_rules);
    out_rules.extend(c.out_rules);
  }

  (
    FSMRet {
      module: None,
      in_rules,
      out_rules,
    },
    children,
  )
}

fn check_ios(
  op: &ir::Op,
  io_in: &IndexSet<ir::ValueId>,
  io_out: &IndexSet<ir::ValueId>,
  in_res: &mut IndexSet<ir::ValueId>,
  out_res: &mut IndexSet<ir::ValueId>,
) {
  for sub in op.sub_ops() {
    check_ios(sub, io_in, io_out, in_res, out_res);
  }
  for v in op.values() {
    if io_in.contains(&v) {
      in_res.insert(v);
    }
    if io_out.contains(&v) {
      out_res.insert(v);
    }
  }
}
