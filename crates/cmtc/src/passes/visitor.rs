use std::{
  arch::x86_64::{_mm_inserti_si64, _MM_FROUND_NO_EXC},
  cmp::Ordering,
  collections::{BinaryHeap, HashSet, VecDeque},
  fmt::Debug,
  process::exit,
};

use cmtir::CmtError;

use crate::RuleRel;

use super::*;

// module A {  rule ra{}; rule rb{}; }  <"A", {<("ra", "rb"), C>}>
type RuleRelMapT = HashMap<String, HashMap<(String, String), ir::MethodRel>>;
/// Contains information passed to visitor functions.
/// This is re-generated when beginning to visit each module.
pub struct VisitorData<'modu> {
  pub circuit: &'modu mut ir::Circuit,
  pub module: ir::Module,
  pub module_idx: usize,
  pub rule: Option<ir::Rule>,
  pub rule_backup: Vec<ir::Rule>,
  pub rule_rel_map: &'modu mut RuleRelMapT,
  pub conflict_map: HashMap<(String, String), ir::MethodRel>,
  pub function: Option<ir::Function>,
}

impl<'modu> VisitorData<'modu> {
  pub fn new_module(
    circuit: &'modu mut ir::Circuit,
    module_name: &str,
    rule_rel_map: &'modu mut RuleRelMapT,
  ) -> Self {
    let module_idx = circuit.module_idx(module_name);
    let mut module = ir::Module::with_name(module_name.to_string());
    std::mem::swap(&mut circuit.modules[module_idx], &mut module);
    Self {
      circuit,
      module,
      module_idx,
      rule: None,
      rule_backup: vec![],
      rule_rel_map,
      function: None,
      conflict_map: HashMap::new(),
    }
  }

  pub fn backup_rules(&mut self) {
    self.rule_backup = self.module.rules().cloned().collect();
  }

  pub fn pop_backup_rules(&mut self) {
    std::mem::swap(self.module.rules_mut(), &mut self.rule_backup);
  }

  pub fn rule_output_types(
    &self,
    module_name: &str,
    rule_name: &str,
  ) -> Result<Vec<ir::Type>, anyhow::Error> {
    log::debug!(
      "try get rule_output_types, module_name: {}, self.module.name(): {}",
      module_name,
      self.module.name()
    );
    Ok(if module_name == self.module.name() {
      self
        .rule_backup
        .iter()
        .find(|r| r.name() == rule_name)
        .ok_or(self.report_error(format!(
          "rule {} not found in module {}",
          rule_name,
          self.module.name()
        )))?
        .outputs()
        .iter()
        .map(|id| self.type_of(*id).unwrap())
        .collect()
    } else {
      let module = self.circuit.module(module_name).ok_or(
        self.report_error(format!("module {} not found", module_name)),
      )?;
      module
        .rule_by_name(rule_name)
        .ok_or(self.report_error(format!(
          "rule {} not found in module {}",
          rule_name, module_name
        )))?
        .outputs()
        .iter()
        .map(|id: &ir::ValueId| {
          module.values.get(*id).unwrap().ty.clone().unwrap()
        })
        .collect()
    })
  }

  pub fn rule_id(&self) -> RuleId {
    RuleId::from(&self.module.name(), &self.rule.as_ref().unwrap().name)
  }

  pub fn rule_id_by_rule_name(&self, rule_name: &str) -> RuleId {
    RuleId {
      module_name: self.module.name().to_string(),
      rule_name: rule_name.to_string(),
    }
  }

  pub fn rule_id_by_inst_rule(&self, inst_rule: &ir::InstRule) -> RuleId {
    RuleId {
      module_name: self.resolve_path(&inst_rule.path),
      rule_name: inst_rule.rule_name.clone(),
    }
  }

  pub fn print_value_in_module(
    &self,
    value_id: ir::ValueId,
    module: &ir::Module,
  ) -> String {
    let value = module.values.get(value_id).unwrap();
    if value.name.is_some() {
      value.name.clone().unwrap()
    } else {
      format!("v{}", value_id)
    }
  }

  pub fn print_value(&self, value_id: ir::ValueId) -> String {
    let value = self.module.values.get(value_id).unwrap();
    if value.name.is_some() {
      value.name.clone().unwrap()
    } else {
      format!("v{}", value_id)
    }
  }

  pub fn current_module_has_input(&self, value_id: ir::ValueId) -> bool {
    self.module.has_input(value_id)
  }

  // given a instance path, return the module name
  pub fn resolve_path(&self, path: &[String]) -> String {
    self.resolve_path_as_module(path).name().to_string()
  }

  pub fn resolve_path_as_module(&self, path: &[String]) -> &ir::Module {
    let mut module_ref = &self.module;
    for segment in path {
      if segment == "self" {
        continue;
      }
      module_ref = match module_ref
        .instances()
        .iter()
        .find(|inst| &inst.name == segment)
      {
        Some(ir::InstDef { module, .. }) => self
          .circuit
          .module(module)
          .expect(format!("cannot find module {}", module).as_str()),
        None => panic!(
          "module {} has no instance named {}, path is {}",
          module_ref.name(),
          segment,
          path.join(".")
        ),
      }
    }
    module_ref
  }

  // return all invoked sub-inst-rules in the inst-rule
  pub fn sub_inst_rules_from(
    &self,
    inst_rule: &ir::InstRule,
  ) -> Vec<ir::InstRule> {
    let module_ref = self.resolve_path_as_module(&inst_rule.path);
    let rule_name = inst_rule.rule_name.clone();

    // find all invokes in the rule
    let rule = module_ref
      .rules()
      .find(|r| r.name() == rule_name)
      .expect(format!("cannot find rule {}", rule_name).as_str());
    let sub_inst_rules = EmptyVisitor::filter_op(rule.ops(), |op| {
      matches!(op.inner(), ir::OpEnum::Invoke(_))
    })
    .into_iter()
    .map(|op| op.as_invoke_op_ref().unwrap().inst_rule.clone())
    .map(|sub_inst_rule| sub_inst_rule.pre_extend(&inst_rule.path))
    .collect();
    sub_inst_rules
  }

  // get the rule from the rule_id
  pub fn rule_from_rule_id(&self, rule_id: &RuleId) -> Option<&ir::Rule> {
    log::debug!("try get rule from rule_id: {:#?}", rule_id);
    if rule_id.module_name == self.module.name() {
      self
        .rule_backup
        .iter()
        .find(|r| r.name() == rule_id.rule_name)
    } else {
      self
        .circuit
        .module(rule_id.module_name.as_str())
        .unwrap()
        .rule_by_name(rule_id.rule_name.as_str())
    }
  }

  pub fn rule_mut_from_rule_name(
    &mut self,
    rule_name: &str,
  ) -> Option<&mut ir::Rule> {
    self
      .module
      .rules_mut()
      .iter_mut()
      .find(|r| r.name() == rule_name)
  }

  pub fn rule_args(
    &self,
    rule_id: Option<&RuleId>,
  ) -> Option<Vec<ir::ValueId>> {
    if let Some(rule_id) = rule_id {
      let rule = self.rule_from_rule_id(rule_id)?;
      Some(rule.inputs().to_vec())
    } else {
      Some(self.rule().inputs())
    }
  }

  pub fn rule_res(&self, rule_id: Option<&RuleId>) -> Option<Vec<ir::ValueId>> {
    if let Some(rule_id) = rule_id {
      let rule = self.rule_from_rule_id(rule_id)?;
      Some(rule.outputs().to_vec())
    } else {
      Some(self.rule().outputs().to_vec())
    }
  }

  /// get the type of a value, return None if the type is not yet inferred
  pub fn type_of(&self, value_id: ir::ValueId) -> Option<ir::Type> {
    self
      .module
      .values
      .get(value_id)
      .expect("the value must exist")
      .ty
      .clone()
  }

  pub fn type_of_in_module(&self, value_id: ir::ValueId, module: &ir::Module) -> Option<ir::Type> {
    module.values.get(value_id).expect("the value must exist").ty.clone()
  }

  pub fn set_type(&mut self, value_id: ir::ValueId, ty: ir::Type) {
    let value = self
      .module
      .values
      .get_mut(value_id)
      .expect("the value must exist");
    value.ty = Some(ty);
  }

  pub fn rule(&self) -> &ir::Rule {
    self.rule.as_ref().expect("no rule is visiting")
  }

  pub fn rule_mut(&mut self) -> &mut ir::Rule {
    self.rule.as_mut().expect("no rule is visiting")
  }

  pub fn take_rule(&mut self) -> ir::Rule {
    self.rule.take().unwrap()
  }

  pub fn report_error_at_span(
    &self,
    msg: String,
    span: Option<ir::MySpan>,
  ) -> CmtError {
    let mut error = CmtError::new();
    error.append(msg, span);
    if let Some(rule) = self.rule.as_ref() {
      for span in rule.span() {
        error.append(format!("in rule: {}", rule.name()), Some(span));
      }
    }
    for span in self.module.spans() {
      error.append(format!("in module: {}", self.module.name()), Some(span));
    }
    error
  }

  pub fn report_error_at_op(&self, msg: String, op: &ir::Op) -> CmtError {
    self.report_error_at_span(msg, op.span())
  }

  pub fn report_error(&self, msg: String) -> CmtError {
    let mut error = CmtError::new();
    error.append(msg, None);
    if let Some(rule) = self.rule.as_ref() {
      for span in rule.span() {
        error.append(format!("in rule: {}", rule.name()), Some(span));
      }
    }
    for span in self.module.spans() {
      error.append(format!("in module: {}", self.module.name()), Some(span));
    }
    error
  }

  pub fn create_wire(
    &mut self,
    value_id: ir::ValueId,
    ty: ir::Type,
  ) -> ir::InstDef {
    let wire_module = builtin::wire_gen(ty);
    let inst_def = ir::InstDef {
      name: format!("wire_{}", value_id),
      module: wire_module.name().to_string(),
    };
    self
      .module
      .add_instance(inst_def.name.clone(), inst_def.module.clone());

    if self.circuit.module(wire_module.name()).is_none() {
      // add the wire module to the circuit
      self.circuit.add_module(wire_module);
    }
    inst_def
  }
}

impl Drop for VisitorData<'_> {
  fn drop(&mut self) {
    std::mem::swap(
      // [Fixed] this is a bug! When a module is taken out, the circuit cannot
      // find it anymore!
      &mut self.circuit.modules[self.module_idx],
      &mut self.module,
    );
  }
}

#[derive(Clone)]
pub struct VisitorArchieve(RuleRelMapT);

impl VisitorArchieve {
  pub fn new() -> Self {
    Self(HashMap::new())
  }

  pub fn join(&mut self, other: Self) {
    self.0.extend(other.0.into_iter());
  }
}

pub type ModuleGraph = DiGraph<String, ()>;

pub type FunctionGraph = DiGraph<String, ()>;

pub trait Visitor
where
  Self: Sized,
{
  fn name() -> &'static str;

  fn skip_tb() -> bool {
    false
  }

  fn filter_item<'a, T>(
    items: impl Iterator<Item = T>,
    filter_fn: impl Fn(&T) -> bool,
  ) -> Vec<T> {
    let mut filtered = Vec::new();
    for item in items {
      if filter_fn(&item) {
        filtered.push(item);
      }
    }
    filtered
  }

  fn filter_op<'a>(
    ops: impl Iterator<Item = &'a ir::Op>,
    filter_fn: impl Fn(&ir::Op) -> bool,
  ) -> Vec<&'a ir::Op> {
    let mut filtered = Vec::new();
    let mut stack = ops
      .collect::<Vec<_>>()
      .into_iter()
      .rev()
      .collect::<Vec<_>>();

    while let Some(op) = stack.pop() {
      stack.extend(op.sub_ops());
      if filter_fn(op) {
        filtered.push(op);
      }
    }
    filtered
  }

  fn flatten_op<'a>(op: &'a ir::Op) -> Vec<&'a ir::Op> {
    let mut flattened = Vec::new();
    let mut stack = vec![(op, false)];

    while let Some((op, second)) = stack.pop() {
      if !second {
        stack.push((op, true));
        stack.extend(
          op.sub_ops()
            .map(|op| (op, false))
            .collect::<Vec<_>>()
            .into_iter()
            .rev(),
        );
      } else {
        flattened.push(op);
      }
    }
    flattened
  }

  fn flatten_ops<'a>(ops: impl Iterator<Item = &'a ir::Op>) -> Vec<&'a ir::Op> {
    let mut flattened = Vec::new();
    for op in ops {
      flattened.extend(Self::flatten_op(op));
    }
    flattened
  }

  fn filter_map<'a, T: Clone + 'a, P>(
    items: impl Iterator<Item = &'a T>,
    filter_fn: impl Fn(&T) -> Option<P>,
  ) -> (Vec<P>, HashMap<usize, usize>) {
    let mut filtered = Vec::new();
    let mut map = HashMap::new();
    for (i, item) in items.enumerate() {
      if let Some(p) = filter_fn(item) {
        filtered.push(p);
        map.insert(i, filtered.len() - 1);
      }
    }
    (filtered, map)
  }

  fn dump_results(&self, data: &mut VisitorData) {
    let _ = data;
  }

  fn visit_op_impl(
    &mut self,
    op: &mut ir::Op,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let _ = op;
    let _ = data;
    Ok(())
  }

  fn visit_op(
    &mut self,
    op: &mut ir::Op,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    self.visit_op_impl(op, data)?;
    for o in op.sub_ops_mut() {
      self.visit_op(o, data)?;
    }
    Ok(())
  }

  fn visit_rule_prelog(
    &mut self,
    rule: &ir::Rule,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let _ = data;
    log::debug!("\tvisiting rule: {}", rule.name());
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    let mut rule = data.take_rule();
    for op in &mut rule.guard_ops {
      self.visit_op(op, data)?;
    }
    for op in &mut rule.ops {
      self.visit_op(op, data)?;
    }
    Ok((vec![rule], vec![]))
  }

  fn visit_rule_guard(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let _ = data;
    Ok(())
  }

  fn visit_rule(
    &mut self,
    rule: ir::Rule,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    self.visit_rule_prelog(&rule, data)?;

    data.rule = Some(rule);

    self.visit_rule_guard(data)?;

    self.visit_rule_impl(data)
  }

  fn topo_sort_rules<'a, T: Clone + Debug>(
    rules: impl Iterator<Item = &'a ir::Rule> + Clone,
    f: impl Fn(usize, &'a ir::Rule) -> (String, T),
  ) -> Vec<T> {
    let mut rule_graph =
      TopoSolver::new(rules.clone().enumerate().map(|(i, r)| f(i, r)));

    for rule in rules {
      for invoke in Self::filter_op(rule.ops().chain(rule.guard()), |op| {
        matches!(op.inner(), ir::OpEnum::Invoke(_))
      }) {
        if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) =
          invoke.inner()
        {
          if inst_rule.is_self() {
            rule_graph.add_edge(
              inst_rule.rule_name.to_string(),
              rule.name().to_string(),
            );
          }
        } else {
          unreachable!()
        }
      }
    }

    rule_graph.topo_sort()
  }

  /// this is very dangerous to use in get_rules_in_order, since the topo order
  /// doesn't guarentee any original order
  fn get_rules_in_topo_order(
    &mut self,
    data: &mut VisitorData,
  ) -> Vec<ir::Rule> {
    let mut rules = std::mem::take(data.module.rules_mut());

    let ordered_rules =
      Self::topo_sort_rules(rules.iter(), |i, r| (r.name().to_string(), i));
    ordered_rules
      .into_iter()
      .map(|i| rules[i].clone())
      .collect()
  }

  fn get_rules_in_order(&mut self, data: &mut VisitorData) -> Vec<ir::Rule> {
    let mut rules = std::mem::take(data.module.rules_mut());

    // default rule order is their order in the vector
    rules
  }

  fn infer_rule_relations(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let mut rel_map: HashMap<(String, String), ir::MethodRel> = HashMap::new();
    for rule_rel in data.module.method_rels() {
      let rule_rel = rule_rel.clone();
      if let RuleRel::Method { rel, lhs, rhs } = rule_rel {
        for lhs in lhs {
          for rhs in rhs.iter() {
            rel_map.insert(
              (lhs.rule_name.clone(), rhs.rule_name.clone()),
              rel.clone(),
            );
            rel_map.insert(
              (rhs.rule_name.clone(), lhs.rule_name.clone()),
              rel.clone().rev(),
            );
          }
        }
      } else {
        unreachable!()
      }
    }

    let mut rule_backup: Vec<ir::Rule> = data.module.rules().cloned().collect();

    let mut rule_list = self.get_rules_in_topo_order(data);
    let num_rules = rule_list.len();
    // for every pair of rules, infer their relations
    for i in 0..num_rules {
      let lhs = &rule_list[i];
      for j in i..num_rules {
        let rhs = &rule_list[j];
        if rel_map
          .contains_key(&(lhs.name().to_string(), rhs.name().to_string()))
        {
          continue;
        }

        let lhs_invokes = Self::filter_op(lhs.ops(), |op| {
          matches!(op.inner(), ir::OpEnum::Invoke(_))
        })
        .into_iter()
        .map(|op| op.as_invoke_op_ref().unwrap().inst_rule.clone())
        .collect::<Vec<_>>();
        let rhs_invokes = Self::filter_op(rhs.ops(), |op| {
          matches!(op.inner(), ir::OpEnum::Invoke(_))
        })
        .into_iter()
        .map(|op| op.as_invoke_op_ref().unwrap().inst_rule.clone())
        .collect::<Vec<_>>();

        let mut rel = ir::MethodRel::CF;

        for lhs_invoke in lhs_invokes.iter() {
          for rhs_invoke in rhs_invokes.iter() {
            if lhs_invoke.path == rhs_invoke.path {
              let module_name = data.resolve_path(&lhs_invoke.path);
              rel = rel.join(
                *if module_name == data.module.name() {
                  &rel_map
                } else {
                  if let Some(rel_map) = data.rule_rel_map.get(&module_name) {
                    rel_map
                  } else {
                    log::error!("no rel-map for {}", module_name);
                    log::error!("existing rel-maps: {:#?}", data.rule_rel_map);
                    exit(0);
                  }
                }
                .get(&(
                  lhs_invoke.rule_name.to_string(),
                  rhs_invoke.rule_name.to_string(),
                ))
                .unwrap_or(&ir::MethodRel::CF),
              );
            }
          }
        }

        rel_map.insert((lhs.name().to_string(), rhs.name().to_string()), rel);
        rel_map
          .insert((rhs.name().to_string(), lhs.name().to_string()), rel.rev());
      }
    }

    data
      .rule_rel_map
      .insert(data.module.name().to_string(), rel_map);

    // log::debug!("infer_rule_relations done {}", data.module.name());
    std::mem::swap(data.module.rules_mut(), &mut rule_backup);
    Ok(())
  }

  fn prepare_visit_module_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let _ = data;
    Ok(())
  }

  fn prepare_visit_module(
    &mut self,
    mut data: VisitorData,
  ) -> Result<(), anyhow::Error> {
    if Self::skip_tb() && data.module.is_tb() {
      return Ok(());
    }
    self.prepare_visit_module_impl(&mut data)?;
    Ok(())
  }

  fn before_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    data.backup_rules();
    Ok(())
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    let _ = data;
    Ok(())
  }

  fn after_pass(
    &mut self,
    circuit: &mut ir::Circuit,
  ) -> Result<(), anyhow::Error> {
    let _ = circuit;
    Ok(())
  }

  fn visit_module(
    &mut self,
    mut data: VisitorData,
  ) -> Result<(), anyhow::Error> {
    if data.module.is_virtual() {
      log::debug!("skip virtual module: {}", data.module.name);
      return Ok(());
    }

    if Self::skip_tb() && data.module.is_tb() {
      log::debug!("skip tb module: {}", data.module.name);
      return Ok(());
    }

    log::debug!("visiting module: {}", data.module.name);

    self.before_visit_rules(&mut data)?;

    // visit rules

    // 1. take out all rules
    // let rules = std::mem::take(data.module.rules_mut());
    let rules = self.get_rules_in_order(&mut data);
    log::debug!(
      "\tto visit rules in order: {}",
      rules
        .iter()
        .map(|r| r.name())
        .collect::<Vec<_>>()
        .join(", ")
    );

    // 2. visit each rule
    let visited_rules = rules
      .into_iter()
      .map(|rule| self.visit_rule(rule, &mut data))
      .collect::<Result<Vec<(Vec<ir::Rule>, Vec<ir::RuleRel>)>, anyhow::Error>>(
      )?;

    // 3. add rules back to the module
    let (rules, rule_rels): (Vec<_>, Vec<_>) =
      visited_rules.into_iter().unzip();
    data.module.rules_mut().extend(rules.into_iter().flatten());
    data
      .module
      .rule_rels_mut()
      .extend(rule_rels.into_iter().flatten());
    self.after_visit_rules(&mut data)?;

    self.dump_results(&mut data);

    Ok(())
  }

  fn pass_prelog(
    &mut self,
    circuit: &mut ir::Circuit,
  ) -> Result<(), anyhow::Error> {
    let _ = circuit;
    Ok(())
  }

  fn visit_function(
    &mut self,
    function: &mut ir::Function,
    circuit: &mut ir::Circuit,
  ) -> Result<(), anyhow::Error> {
    let _ = function;
    let _ = circuit;
    Ok(())
  }

  fn after_visit_modules(
    &mut self,
    circuit: &mut ir::Circuit,
  ) -> Result<(), anyhow::Error> {
    let _ = circuit;
    Ok(())
  }

  /// apply the pass to the every module in the circuit
  fn apply_pass(
    &mut self,
    circuit: &mut ir::Circuit,
  ) -> Result<(), anyhow::Error> {
    log::info!("applying pass {:?}", Self::name());
    let function_order: Vec<String> = visit_function_order(circuit);
    for function_name in function_order.iter() {
      let function_idx = circuit.function_idx(function_name);
      let mut function = std::mem::take(&mut circuit.functions[function_idx]);
      self.visit_function(&mut function, circuit)?;
      circuit.functions[function_idx] = function;
    }

    // let module_graph = build_module_graph(circuit)?;
    let visit_order = visit_order(circuit);
    // log::debug!("module-graph: {:#?}", module_graph);
    // log::debug!("visit-order: {:?}", visit_order);
    // let dotfile_dir = PathBuf::from("./module-graph.tmp.dot");
    // export the module graph to a dot file
    // let dot = petgraph::dot::Dot::new(&module_graph);
    // log::debug!("module-graph exported to {}", dotfile_dir.display());
    // std::fs::write(dotfile_dir, format!("{:?}", dot)).unwrap();

    self.pass_prelog(circuit)?;

    // let mut acc_data = VisitorArchieve::new();
    let mut rule_rel_map = HashMap::new();
    // preparation round: visit modules in order without visiting rules
    for module_name in visit_order.iter() {
      let mut visitor_data =
        VisitorData::new_module(circuit, module_name, &mut rule_rel_map);
      self.prepare_visit_module(visitor_data)?;
      // log::debug!(
      //   "preparation round done for {}, rule_rel_map: {:#?}",
      //   module_name,
      //   rule_rel_map
      // );
    }

    // visit modules according to the visit_order
    for module_name in visit_order.iter() {
      let mut visitor_data =
        VisitorData::new_module(circuit, module_name, &mut rule_rel_map);
      self.visit_module(visitor_data)?;
    }

    self.after_visit_modules(circuit)?;

    self.after_pass(circuit)?;

    // log::info!(
    //   "\tafter pass modules: {}",
    //   circuit
    //     .module_names()
    //     .map(|m| m.to_string())
    //     .collect::<Vec<_>>()
    //     .join(", ")
    // );

    Ok(())
  }
}

/// build a module graph from the circuit, where an edge from module B to
/// module A exists if module A has an instance of module B. this is used to
/// determine the order in which modules should be visited
fn build_module_graph(circuit: &ir::Circuit) -> Result<ModuleGraph, String> {
  let mut topo_solver = TopoSolver::new(
    circuit
      .module_names()
      .map(|module| (module.to_string(), module.to_string())),
  );

  // add edges between modules that have instances of each other
  for module in circuit.modules().filter(|m| m.is_native()) {
    for ir::InstDef {
      module: sub_module, ..
    } in module.instances()
    {
      if !topo_solver.mapping.contains_key(sub_module) {
        log::warn!(
          "module {} has an instance of undefined module {}",
          module.name,
          sub_module
        );
        // return Err(format!(
        //   "module {} has an instance of undefined module {}",
        //   module.name, sub_module
        // ));
      } else {
        topo_solver.add_edge(sub_module.to_string(), module.name.to_string());
      }
    }
  }

  let module_graph = topo_solver.digraph;

  Ok(module_graph)
}

fn build_function_graph(
  circuit: &ir::Circuit,
) -> Result<FunctionGraph, String> {
  let mut topo_solver = TopoSolver::new(
    circuit
      .function_names()
      .map(|function| (function.to_string(), function.to_string())),
  );

  // add edges between modules that have instances of each other
  for function in circuit.functions() {
    for op in function.ops.iter() {
      if let ir::OpEnum::Call(ir::CallOp {
        function_name: func_name,
        ..
      }) = op.inner()
      {
        if !topo_solver.mapping.contains_key(func_name) {
          log::warn!(
            "{} call undefined function {}",
            function.name(),
            func_name
          );
        } else {
          topo_solver
            .add_edge(func_name.to_string(), function.name().to_string());
        }
      }
    }
  }

  let function_graph = topo_solver.digraph;

  Ok(function_graph)
}

pub fn visit_order(circuit: &ir::Circuit) -> Vec<String> {
  let module_graph = build_module_graph(circuit).unwrap();
  let topo_order = petgraph::algo::toposort(&module_graph, None).unwrap();
  topo_order
    .into_iter()
    .map(|node| module_graph.node_weight(node).unwrap().to_string())
    .collect()
}

fn visit_function_order(circuit: &ir::Circuit) -> Vec<String> {
  let function_graph = build_function_graph(circuit).unwrap();
  let topo_order = petgraph::algo::toposort(&function_graph, None).unwrap();
  topo_order
    .into_iter()
    .map(|node| function_graph.node_weight(node).unwrap().to_string())
    .collect()
}

pub struct EmptyVisitor;
impl Visitor for EmptyVisitor {
  fn name() -> &'static str {
    "empty-visitor"
  }
}

impl VisitorData<'_> {
  pub fn topo_sort_ops(&self, ops: Vec<ir::Op>) -> Vec<ir::Op> {
    let mut op_graph =
      TopoSolver::new(ops.iter().enumerate().map(|(i, _)| (i.to_string(), i)));
    let mut def_map = HashMap::new();
    let mut op_to_idx = HashMap::new();
    for (idx, op) in ops.iter().enumerate() {
      op_to_idx.insert(op.clone(), idx);
      if !op.is_return() {
        for output in op.outputs() {
          def_map.insert(output, op.clone());
        }
      }
    }
    for (idx, op) in ops.iter().enumerate() {
      if op.is_return() {
        for output in op.outputs() {
          if let Some(op_def) = def_map.get(&output) {
            let idx_2 = op_to_idx.get(op_def).unwrap();
            op_graph.add_edge(idx_2.to_string(), idx.to_string());
          }
        }
      } else {
        for input in op.inputs() {
          if let Some(op_def) = def_map.get(&input) {
            let idx_2 = op_to_idx.get(op_def).unwrap();
            op_graph.add_edge(idx_2.to_string(), idx.to_string());
          }
        }
      }
    }
    let topo_order = op_graph.topo_sort();
    let ordered_op = topo_order.iter().map(|i| ops[*i].clone()).collect();
    ordered_op
  }

  pub fn schedule_rules(
    &self,
    rules_to_schedule: Vec<InstRule>,
  ) -> Result<Vec<InstRule>, anyhow::Error> {
    // sort all_rules by schedules from data.module
    // the first priority is the schedule order
    //   for any schedule [r0, r1, ..., rn], we have r0 < r1 < ... < rn
    // the second priority is the original rule order
    //   for all_rules = [r0, r1, ..., rn], we have r0 < r1 < ... < rn
    // let mut sched_constraints = HashMap::new();

    log::debug!("schedule rules: {:#?}", rules_to_schedule);
    let mut rules_contain = rules_to_schedule
      .clone()
      .into_iter()
      .collect::<HashSet<_>>();
    let mut inst_idx_map = HashMap::new();
    let mut prev_map = HashMap::new();
    let mut next_map = HashMap::new();

    for schedule in self.module.schedules() {
      log::debug!("schedule: {:#?}", schedule);
      let mut iter =
        schedule.iter().filter(|rule| rules_contain.contains(rule));

      let mut prev_rule = if let Some(rule) = iter.next() {
        vec![rule.clone()]
      } else {
        continue;
      };

      for rule in iter {
        if rules_contain.contains(&rule) {
          for prev in prev_rule.iter() {
            next_map
              .entry(prev.to_string())
              .or_insert(HashSet::new())
              .insert(rule.clone());
            prev_map
              .entry(rule.to_string())
              .or_insert(HashSet::new())
              .insert(prev.to_string());
            // log::debug!("add edge {} -> {}", rule.to_string(),
            // prev.to_string());
          }

          prev_rule.push(rule.clone());
        }
      }
    }

    let mut rules_with_idx = rules_to_schedule
      .into_iter()
      .enumerate()
      .collect::<Vec<_>>();

    let mut sorted_rules = vec![];
    let mut free_queue = BinaryHeap::new();

    for (idx, inst_rule) in rules_with_idx.iter() {
      if prev_map.get(&inst_rule.to_string()).is_none() {
        log::debug!("rule {} is free", inst_rule.to_string());
        free_queue.push(IdxInstRule {
          idx: *idx,
          inst_rule: inst_rule.clone(),
        });
      } else {
        // log::debug!(
        //   "rule {} is not free with prefix: {:#?}",
        //   inst_rule.to_string(),
        //   prev_map.get(&inst_rule.to_string()).unwrap()
        // );
        inst_idx_map.insert(inst_rule.to_string(), *idx);
      }
    }

    while !free_queue.is_empty() {
      let IdxInstRule { idx: _, inst_rule } = free_queue.pop().unwrap();
      // log::debug!("add {} to schedule", inst_rule.to_string());
      sorted_rules.push(inst_rule.clone());
      if let Some(next_rules) = next_map.get(&inst_rule.to_string()) {
        for next_rule in next_rules {
          prev_map
            .get_mut(&next_rule.to_string())
            .unwrap()
            .remove(&inst_rule.to_string());
          if prev_map.get(&next_rule.to_string()).unwrap().is_empty() {
            // log::debug!("rule {} is free", next_rule.to_string());
            free_queue.push(IdxInstRule {
              idx: inst_idx_map.remove(&next_rule.to_string()).expect(
                &format!(
                  "rule {} not found in inst_idx_map: {:#?}",
                  next_rule.to_string(),
                  inst_idx_map
                ),
              ),
              inst_rule: next_rule.clone(),
            });
          }
        }
      }
    }

    if inst_idx_map.len() != 0 {
      // find a rule cycle by bfs
      let mut vq = VecDeque::new();
      let mut last_ptr: HashMap<String, String> = HashMap::new();
      let mut visited = HashSet::new();
      vq.push_back(inst_idx_map.keys().next().unwrap().to_string());

      let mut cycle_str = String::new();
      let mut cycle_found = false;
      while !vq.is_empty() && !cycle_found {
        let rule = vq.pop_front().unwrap();
        if let Some(prev_rules) = prev_map.get(&rule) {
          for prev_rule in prev_rules {
            if visited.contains(prev_rule) {
              // found a cycle
              cycle_str.push_str(prev_rule);
              let mut cur = rule;
              while let Some(last) = last_ptr.get(&cur) {
                // log::debug!("trace back to rule {}", last);
                cycle_str.push_str(&format!(" -> {}", last));
                if last == prev_rule {
                  break;
                }
                cur = last.to_string();
              }
              cycle_found = true;
              break;
            } else {
              last_ptr.insert(prev_rule.to_string(), rule.to_string());
              visited.insert(prev_rule.to_string());
              vq.push_back(prev_rule.to_string());
            }
          }
        }
      }

      return Err(self.report_error(format!(
        "some rules are not scheduled\n\tCycle found: {}",
        cycle_str
      )))?;
    } else {
      Ok(sorted_rules)
    }
  }
}

#[derive(Clone, Eq, PartialEq)]
struct IdxInstRule {
  idx: usize,
  inst_rule: InstRule,
}

impl Ord for IdxInstRule {
  fn cmp(&self, other: &Self) -> Ordering {
    self.idx.cmp(&other.idx).reverse()
  }
}

impl PartialOrd for IdxInstRule {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
