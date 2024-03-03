use std::collections::{hash_map, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use visible::StructFields;

pub use super::arena::*;

mod display;
pub use display::*;

mod iter;
pub use iter::*;

#[StructFields(pub)]
#[derive(Debug, Clone)]
pub struct Node<NDataT> {
  idx: NodeIndex,
  data: NDataT,
  in_edges: HashSet<EdgeIndex>,
  out_edges: HashSet<EdgeIndex>,
}

#[StructFields(pub)]
#[derive(Debug, Clone)]
pub struct Edge<EDataT> {
  idx: EdgeIndex,
  data: EDataT,
  from: NodeIndex,
  to: NodeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(pub usize);

impl NodeIndex {
  pub fn empty() -> NodeIndex { NodeIndex(0) }

  pub fn is_empty(&self) -> bool { self.0 == 0 }
}

impl ArenaIndex for NodeIndex {
  fn new(id: usize) -> Self { NodeIndex(id) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeIndex(pub usize);
impl ArenaIndex for EdgeIndex {
  fn new(id: usize) -> Self { EdgeIndex(id) }
}

impl EdgeIndex {
  pub fn empty() -> EdgeIndex { EdgeIndex(0) }

  pub fn is_empty(&self) -> bool { self.0 == 0 }
}

#[derive(Debug, Clone)]
pub struct Graph<NDataT, EDataT> {
  nodes: Arena<Node<NDataT>, NodeIndex>,
  edges: Arena<Edge<EDataT>, EdgeIndex>,
}

impl<NDataT, EDataT> Graph<NDataT, EDataT> {
  pub fn new(context: &Context) -> Graph<NDataT, EDataT> {
    Graph {
      nodes: Arena::new(Arc::clone(&context.node_dist)),
      edges: Arena::new(Arc::clone(&context.edge_dist)),
    }
  }

  pub fn get_node(&self, idx: NodeIndex) -> Option<&Node<NDataT>> { self.nodes.get(idx) }

  pub fn get_edge(&self, idx: EdgeIndex) -> Option<&Edge<EDataT>> { self.edges.get(idx) }

  pub fn iter_nodes(&self) -> hash_map::Iter<'_, NodeIndex, Node<NDataT>> {
    self.nodes.iter()
  }

  pub fn iter_edges(&self) -> hash_map::Iter<'_, EdgeIndex, Edge<EDataT>> {
    self.edges.iter()
  }

  pub fn iter_in(&self, node: NodeIndex) -> EdgeIterator<'_, '_, EDataT> {
    EdgeIterator {
      edges: &self.edges,
      iter: self.nodes.get(node).unwrap().in_edges.iter(),
    }
  }

  pub fn iter_out(&self, node: NodeIndex) -> EdgeIterator<'_, '_, EDataT> {
    EdgeIterator {
      edges: &self.edges,
      iter: self.nodes.get(node).unwrap().out_edges.iter(),
    }
  }

  pub fn len_nodes(&self) -> usize { self.nodes.len() }

  pub fn len_edges(&self) -> usize { self.edges.len() }

  pub fn commit(&mut self, t: Transaction<NDataT, EDataT>) {
    if t.committed {
      return;
    }

    self.merge_nodes(t.inc_nodes);
    self.merge_edges(t.inc_edges);
    for (i, f) in t.mut_nodes {
      self.modify_node(i, f)
    }
    for (i, f) in t.mut_edges {
      self.modify_edge(i, f)
    }
    for (i, f) in t.update_nodes {
      self.update_node(i, f)
    }
    for (i, f) in t.update_edges {
      self.update_edge(i, f)
    }
    for n in &t.dec_nodes {
      self.remove_node(*n);
    }
    for e in &t.dec_edges {
      self.remove_edge(*e);
    }
  }

  fn merge_nodes(&mut self, nodes: Arena<Node<NDataT>, NodeIndex>) {
    self.nodes.merge(nodes);
  }

  fn merge_edges(&mut self, edges: Arena<Edge<EDataT>, EdgeIndex>) {
    for (idx, e) in edges.iter() {
      self.nodes[e.from].out_edges.insert(*idx);
      self.nodes[e.to].in_edges.insert(*idx);
    }
    self.edges.merge(edges);
  }

  fn remove_node(&mut self, idx: NodeIndex) { self.nodes.remove(idx); }

  fn remove_edge(&mut self, idx: EdgeIndex) {
    let edge = self.edges.remove(idx).unwrap();
    self.nodes.get_mut(edge.to).unwrap().in_edges.remove(&idx);
    self.nodes.get_mut(edge.from).unwrap().out_edges.remove(&idx);
  }

  fn modify_node<F>(&mut self, i: NodeIndex, f: F)
  where F: FnOnce(&mut NDataT) {
    f(&mut self.nodes.get_mut(i).unwrap().data);
  }

  fn modify_edge<F>(&mut self, i: EdgeIndex, f: F)
  where F: FnOnce(&mut EDataT) {
    f(&mut self.edges.get_mut(i).unwrap().data);
  }

  fn update_node<F>(&mut self, i: NodeIndex, f: F)
  where F: FnOnce(NDataT) -> NDataT {
    self.nodes.update_with(i, |x| Node { data: f(x.data), ..x });
  }

  fn update_edge<F>(&mut self, i: EdgeIndex, f: F)
  where F: FnOnce(EDataT) -> EDataT {
    self.edges.update_with(i, |x| Edge { data: f(x.data), ..x });
  }
}

pub struct Transaction<'a, NDataT, EDataT> {
  committed: bool,
  inc_nodes: Arena<Node<NDataT>, NodeIndex>,
  inc_edges: Arena<Edge<EDataT>, EdgeIndex>,
  dec_nodes: Vec<NodeIndex>,
  dec_edges: Vec<EdgeIndex>,
  mut_nodes: Vec<(NodeIndex, Box<dyn FnOnce(&mut NDataT) + 'a>)>,
  mut_edges: Vec<(EdgeIndex, Box<dyn FnOnce(&mut EDataT) + 'a>)>,
  update_nodes: Vec<(NodeIndex, Box<dyn FnOnce(NDataT) -> NDataT + 'a>)>,
  update_edges: Vec<(EdgeIndex, Box<dyn FnOnce(EDataT) -> EDataT + 'a>)>,
}

impl<'a, NDataT, EDataT> Transaction<'a, NDataT, EDataT> {
  pub fn new(context: &Context) -> Self {
    let node_dist = Arc::clone(&context.node_dist);
    let edge_dist = Arc::clone(&context.edge_dist);
    Transaction {
      committed: false,
      inc_nodes: Arena::new(node_dist),
      inc_edges: Arena::new(edge_dist),
      dec_nodes: Vec::new(),
      dec_edges: Vec::new(),
      mut_nodes: Vec::new(),
      mut_edges: Vec::new(),
      update_nodes: Vec::new(),
      update_edges: Vec::new(),
    }
  }

  pub fn new_node(&mut self, data: NDataT) -> NodeIndex {
    self.inc_nodes.insert_with(|idx| Node {
      idx,
      data,
      in_edges: HashSet::new(),
      out_edges: HashSet::new(),
    })
  }

  pub fn new_edge(&mut self, data: EDataT, from: NodeIndex, to: NodeIndex) -> EdgeIndex {
    self.inc_edges.insert_with(|idx| Edge { idx, data, from, to })
  }

  pub fn remove_node(&mut self, node: NodeIndex) {
    if self.inc_nodes.remove(node).is_none() {
      self.dec_nodes.push(node);
    }
  }

  pub fn remove_edge(&mut self, edge: EdgeIndex) {
    if self.inc_edges.remove(edge).is_none() {
      self.dec_edges.push(edge);
    }
  }

  pub fn mut_node<F>(&mut self, node: NodeIndex, func: F)
  where F: FnOnce(&mut NDataT) + 'a {
    if self.inc_nodes.contains(node) {
      func(&mut self.inc_nodes.get_mut(node).unwrap().data);
    } else {
      self.mut_nodes.push((node, Box::new(func)));
    }
  }

  pub fn mut_edge<F>(&mut self, edge: EdgeIndex, func: F)
  where F: FnOnce(&mut EDataT) + 'a {
    if self.inc_edges.contains(edge) {
      func(&mut self.inc_edges.get_mut(edge).unwrap().data);
    } else {
      self.mut_edges.push((edge, Box::new(func)));
    }
  }

  pub fn update_node<F>(&mut self, node: NodeIndex, func: F)
  where F: FnOnce(NDataT) -> NDataT + 'a {
    if self.inc_nodes.contains(node) {
      self.inc_nodes.update_with(node, |x| Node { data: func(x.data), ..x });
    } else {
      self.update_nodes.push((node, Box::new(func)));
    }
  }

  pub fn update_edge<F>(&mut self, edge: EdgeIndex, func: F)
  where F: FnOnce(EDataT) -> EDataT + 'a {
    if self.inc_edges.contains(edge) {
      self.inc_edges.update_with(edge, |x| Edge { data: func(x.data), ..x });
    } else {
      self.update_edges.push((edge, Box::new(func)));
    }
  }

  pub fn give_up(&mut self) { self.committed = true; }
}

#[derive(Debug)]
pub struct Context {
  node_dist: Arc<IdDistributer>,
  edge_dist: Arc<IdDistributer>,
}
impl Context {
  pub fn new() -> Context {
    Context {
      node_dist: Arc::new(IdDistributer::new()),
      edge_dist: Arc::new(IdDistributer::new()),
    }
  }
}
impl Clone for Context {
  fn clone(&self) -> Self {
    Context {
      node_dist: Arc::clone(&self.node_dist),
      edge_dist: Arc::clone(&self.edge_dist),
    }
  }
}
