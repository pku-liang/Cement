use std::fmt::Debug;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId {
  pub module_name: String,
  pub rule_name: String,
}

impl ToString for RuleId {
  fn to_string(&self) -> String {
    format!("{}::{}", self.module_name, self.rule_name)
  }
}

impl From<(String, String)> for RuleId {
  fn from(value: (String, String)) -> Self {
    Self {
      module_name: value.0,
      rule_name: value.1,
    }
  }
}

impl RuleId {
  pub fn from(callee_name: &str, rule_name: &str) -> Self {
    Self {
      module_name: callee_name.to_string(),
      rule_name: rule_name.to_string(),
    }
  }
  pub fn to_string(&self) -> String {
    format!("{}::{}", self.module_name, self.rule_name)
  }
}

pub struct TopoSolver<T: Clone> {
  pub digraph: DiGraph<T, ()>,
  pub mapping: HashMap<String, NodeIndex<u32>>,
}

impl<T: Clone + Debug> TopoSolver<T> {
  pub fn new(nodes_with_name: impl IntoIterator<Item = (String, T)>) -> Self {
    let mut digraph = DiGraph::<T, ()>::new();
    let mut mapping = HashMap::new();
    for (name, data) in nodes_with_name {
      let node_index = digraph.add_node(data);
      mapping.insert(name, node_index);
    }
    Self { digraph, mapping }
  }

  pub fn add_edge(&mut self, u: String, v: String) {
    if let Some(u) = self.mapping.get(&u) {
      if let Some(v) = self.mapping.get(&v) {
        self.digraph.add_edge(*u, *v, ());
      } else {
        log::error!("node {} not found", v);
        panic!("node {} not found", v);
      }
    } else {
      log::error!("node {} not found", u);
      panic!("node {} not found", u);
    }
  }

  pub fn topo_sort(&self) -> Vec<T> {
    let topo_order = match petgraph::algo::toposort(&self.digraph, None) {
      Ok(topo_order) => topo_order,
      Err(_e) => {
        let dot = petgraph::dot::Dot::new(&self.digraph);
        log::error!("topo sort failed: \n {:?}", dot);
        panic!("topo sort failed");
      }
    };
    topo_order
      .into_iter()
      .map(|node| self.digraph.node_weight(node).unwrap().clone())
      .collect()
  }
}
