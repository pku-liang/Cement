use super::*;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub struct SideEffectPass {
  graph: DiGraph<(String, String), ()>,
  node_map: HashMap<(String, String), NodeIndex>,
  side_effect_nodes: Vec<(String, String)>,
}

impl SideEffectPass {
  pub fn new() -> Self {
    Self {
      graph: DiGraph::new(),
      node_map: HashMap::new(),
      side_effect_nodes: Vec::new(),
    }
  }

  pub fn add_rule_with_edge(
    &mut self,
    rule1: (String, String),
    rule2: (String, String),
  ) {
    let node1 = *self
      .node_map
      .entry(rule1.clone())
      .or_insert_with(|| self.graph.add_node(rule1));

    let node2 = *self
      .node_map
      .entry(rule2.clone())
      .or_insert_with(|| self.graph.add_node(rule2));

    self.graph.add_edge(node1, node2, ());
  }
}

impl Visitor for SideEffectPass {
  fn name() -> &'static str {
    "SideEffectPass"
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    if data.rule().has_side_effect() {
      self
        .side_effect_nodes
        .push((data.module.name.clone(), data.rule().name.clone()));
    }

    let guard_ops =
      data.rule().guard().map(|op| op.clone()).collect::<Vec<_>>();
    for op in guard_ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
          let module_name = data.resolve_path(&inst_rule.path);
          let rule_name = inst_rule.rule_name.clone();
          self.add_rule_with_edge(
            (module_name, rule_name),
            (data.module.name().to_string(), data.rule().name.clone()),
          );
        }
      }
    }

    let ops = data.rule().ops().map(|op| op.clone()).collect::<Vec<_>>();
    for op in ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
          let module_name = data.resolve_path(&inst_rule.path);
          let rule_name = inst_rule.rule_name.clone();
          self.add_rule_with_edge(
            (module_name, rule_name),
            (data.module.name().to_string(), data.rule().name.clone()),
          );
        }
      }
    }

    // Keep the rule as is
    Ok((vec![data.take_rule()], vec![]))
  }

  fn after_pass(&mut self, circuit: &mut ir::Circuit) -> Result<(), anyhow::Error> {
    let mut visited: HashSet<NodeIndex> = HashSet::new();

    // Iterate through each side effect node in side_effect_nodes
    for side_effect_node in &self.side_effect_nodes {
      if let Some(&start_index) = self.node_map.get(side_effect_node) {
        let mut stack = vec![start_index];
        while let Some(node) = stack.pop() {
          if !visited.insert(node) {
            continue;
          }

          if let Some((module_name, rule_name)) =
            self.graph.node_weight_mut(node)
          {
            let module = circuit
              .modules
              .iter_mut()
              .find(|m| m.name == *module_name)
              .unwrap();
            let rule = module
              .rules_mut()
              .iter_mut()
              .find(|r| r.name == *rule_name)
              .unwrap();
            rule.set_side_effect(true);
          }

          stack.extend(self.graph.neighbors(node));
        }
      }
    }
    Ok(())
  }
}
