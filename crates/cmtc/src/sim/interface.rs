use cmtc::passes::*;
use indexmap::IndexMap;
use std::collections::HashMap;
use cmtir::utils::indent;

use super::*;

#[derive(Clone, Debug)]
pub struct SimData {
  module_table: HashMap<String, ModuleRun>,
}

impl SimData {
  pub fn new() -> Self {
    Self {
      module_table: HashMap::new(),
    }
  }

  pub fn dump(&self) -> String {
    let mut s = String::new();
    for (module_name, module_run) in &self.module_table {
      s.push_str(&format!(
        "module {}: {}\n",
        module_name,
        indent(module_run.to_string(), 2)
      ));
    }
    s
  }

  pub fn module_runs(&self) -> impl Iterator<Item = (&String, &ModuleRun)> {
    self.module_table.iter()
  }

  pub fn module_run(&self, module_name: &str) -> Option<&ModuleRun> {
    self.module_table.get(module_name)
  }
}

#[derive(Clone, Debug)]
pub struct ModuleRun {
  is_tb: bool,
  schedule: Vec<ir::InstRule>,
  instances: Vec<ir::InstDef>,
  rule_table: HashMap<String, RuleRun>,
}

impl ModuleRun {
  pub fn schedule(&self) -> &Vec<ir::InstRule> {
    &self.schedule
  }

  pub fn instances(&self) -> &Vec<ir::InstDef> {
    &self.instances
  }

  pub fn rule_run(&self, rule_name: &str) -> Option<&RuleRun> {
    self.rule_table.get(rule_name)
  }

  pub fn get_all_rule_name(&self) -> Vec<String> {
    self.rule_table.keys().cloned().collect()
  }
}

impl ToString for ModuleRun {
  fn to_string(&self) -> String {
    let mut s = String::new();
    s.push_str(&format!("is_tb: {}\n", self.is_tb));
    s.push_str(&format!(
      "schedule: {}\n",
      self
        .schedule
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join(", ")
    ));
    s.push_str(&format!(
      "instances: {}\n",
      self
        .instances
        .iter()
        .map(|i| i.ir_dump())
        .collect::<Vec<_>>()
        .join(", ")
    ));
    s.push_str(&format!(
      "rule_table: \n{}\n",
      indent(
        self
          .rule_table
          .iter()
          .map(|(k, v)| format!("{}: {}", k, v.to_string()))
          .collect::<Vec<_>>()
          .join("\n"),
        2
      )
    ));
    s
  }
}

#[derive(Clone, Debug)]
pub struct RuleRun {
  ops: Vec<ir::Op>,
  guards: Vec<Guard>,
  is_private: bool,
  inputs: Vec<ValueId>,
  outputs: Vec<ValueId>,
}

impl RuleRun {
  pub fn guards(&self) -> impl Iterator<Item = &Guard> {
    self.guards.iter()
  }

  pub fn ops(&self) -> impl Iterator<Item = &ir::Op> {
    self.ops.iter()
  }

  pub fn is_private(&self) -> bool {
    self.is_private
  }

  pub fn is_always(&self) -> bool {
    self.guards.is_empty() || self.guards[0] != Guard::Invoke
  }

  pub fn get_inputs(&self) -> Vec<ValueId> {
    self.inputs.clone()
  }

  pub fn get_outputs(&self) -> Vec<ValueId> {
    self.outputs.clone()
  }
}

impl ToString for RuleRun {
  fn to_string(&self) -> String {
    format!(
      "guards: {}",
      self
        .guards
        .iter()
        .map(|g| g.to_string())
        .collect::<Vec<_>>()
        .join(", "),
    )
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Guard {
  Invoke, // for methods
  Explicit(ir::Op),
  Implicit(RunStatus),
}

impl ToString for Guard {
  fn to_string(&self) -> String {
    match self {
      Guard::Invoke => "invoke".to_string(),
      Guard::Explicit(op) => format!("explicit: {}", op.kind_str()),
      Guard::Implicit(status) => format!("implicit: {}", status.to_string()),
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunStatus {
  Run(ir::InstRule),
  NotRun(ir::InstRule),
}

impl ToString for RunStatus {
  fn to_string(&self) -> String {
    match self {
      RunStatus::Run(rule) => format!("run({})", rule.to_string()),
      RunStatus::NotRun(rule) => format!("norun({})", rule.to_string()),
    }
  }
}

pub struct SimItfc {
  sim_data: SimData,
}

impl SimItfc {
  pub fn new() -> Self {
    Self {
      sim_data: SimData::new(),
    }
  }

  pub fn into_inner(self) -> SimData {
    self.sim_data
  }

  fn collect_explicit_guards(&self, data: &VisitorData) -> Vec<Guard> {
    let guard_ops =
      data.rule().guard().map(|op| op.clone()).collect::<Vec<_>>();
    // for guard_op in guard_ops {
    //   explicit_guards.push(Guard::Explicit(guard_op));
    // }
    let guard_op_as_block = ir::Op::block(guard_ops);

    vec![Guard::Explicit(guard_op_as_block)]
  }

  fn collect_implicit_guards(&self, data: &mut VisitorData) -> Vec<Guard> {
    let mut implicit_guards = vec![];
    let guard_ops =
      data.rule().guard().map(|op| op.clone()).collect::<Vec<_>>();
    for op in guard_ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
          implicit_guards
            .push(Guard::Implicit(RunStatus::Run(inst_rule.clone())));
        }
      }
    }

    let ops = data.rule().ops().map(|op| op.clone()).collect::<Vec<_>>();
    for op in ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
          implicit_guards
            .push(Guard::Implicit(RunStatus::Run(inst_rule.clone())));
        }
      }
    }

    implicit_guards
  }
}

impl Visitor for SimItfc {
  fn name() -> &'static str {
    "SimItfc"
  }

  fn prepare_visit_module_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    let mut sim_data = self.sim_data.clone();

    let instances: Vec<ir::InstDef> = data
      .module
      .instances()
      .iter()
      .map(|inst| (*inst).clone())
      .collect();
    let schedules = data.module.schedules().next().unwrap_or(vec![]);
    let module_run = ModuleRun {
      is_tb: data.module.is_tb(),
      schedule: schedules,
      instances: instances,
      rule_table: HashMap::new(),
    };
    sim_data
      .module_table
      .insert(data.module.name.clone(), module_run);

    let module_run = sim_data.module_table.get_mut(data.module.name()).unwrap();
    // for instance in data.module.instances() {
    //   // data.instance = Some(instance.clone());
    //   // self.register_instance(&data, instance)?;
    // }

    self.infer_rule_relations(data)?;

    let mut rule_guards = IndexMap::new();

    if let Some(rel_map) = data.rule_rel_map.get(data.module.name()) {
      if rel_map.len() > 500 {
        // for ((rule0, rule1), rel) in rel_map {
        //   eprintln!("{} {} {}", rule0, rel.to_string(), rule1);
        // }
        // eprintln!("module {} has lots of relations", data.module.name());
      //   log::warn!(
      //     "module {} has lots of ({}) relations, with {} rules",
      //     data.module.name(),
      //     rel_map.len(),
      //     data.module.rules.len(),
      //   );
      }
      for ((rule0, rule1), rel) in rel_map {
        if matches!(rel, ir::MethodRel::C | ir::MethodRel::SA) {
          rule_guards.entry(rule0.clone()).or_insert(vec![]).push(
            Guard::Implicit(RunStatus::NotRun(ir::InstRule {
              path: vec![],
              rule_name: rule1.clone(),
            })),
          );
        }
        if matches!(rel, ir::MethodRel::C | ir::MethodRel::SB) {
          rule_guards.entry(rule1.clone()).or_insert(vec![]).push(
            Guard::Implicit(RunStatus::NotRun(ir::InstRule {
              path: vec![],
              rule_name: rule0.clone(),
            })),
          );
        }
      }
    }

    for rule in data.module.clone().rules() {
      data.rule = Some(rule.clone());
      let mut guards = rule_guards.swap_remove(rule.name()).unwrap_or(vec![]);
      guards.extend(self.collect_explicit_guards(&data));
      guards.extend(self.collect_implicit_guards(data));
      if !rule.is_always() {
        guards.insert(0, Guard::Invoke);
      }
      let ops = data
        .rule
        .as_ref()
        .unwrap()
        .ops()
        .map(|op| op.clone())
        .collect::<Vec<_>>();

      let is_private = data.rule.as_ref().unwrap().is_private();
      let inputs = data.rule.as_ref().unwrap().inputs().clone();
      let outputs = data.rule.as_ref().unwrap().outputs().clone();
      let rule_run = RuleRun {
        ops: ops,
        guards: guards.clone(),
        is_private: is_private,
        inputs,
        outputs,
      };
      module_run
        .rule_table
        .insert(rule.name().to_string(), rule_run);
    }

    self.sim_data = sim_data;
    Ok(())
  }
}

