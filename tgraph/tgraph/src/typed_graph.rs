// typed graph

use std::collections::{hash_map, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use uuid::Uuid;

use crate::arena::*;

pub mod debug;
pub mod library;
pub use debug::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(pub usize);

impl NodeIndex {
  pub fn empty() -> NodeIndex { NodeIndex(0) }

  pub fn is_empty(&self) -> bool { self.0 == 0 }
}

impl ArenaIndex for NodeIndex {
  fn new(id: usize) -> Self { NodeIndex(id) }
}

#[derive(Clone)]
pub struct Graph<NodeT: NodeEnum> {
  ctx_id: Uuid,
  nodes: Arena<NodeT, NodeIndex>,
  back_links: HashMap<NodeIndex, HashSet<(NodeIndex, NodeT::SourceEnum)>>,
}

impl<NodeT: NodeEnum> Graph<NodeT> {
  pub fn new(context: &Context) -> Self {
    Graph {
      ctx_id: context.id,
      nodes: Arena::new(Arc::clone(&context.node_dist)),
      back_links: HashMap::new(),
    }
  }

  pub fn get_node(&self, idx: NodeIndex) -> Option<&NodeT> { self.nodes.get(idx) }

  pub fn iter_nodes(&self) -> Iter<'_, NodeT> { self.nodes.iter() }

  pub fn len(&self) -> usize { self.nodes.len() }

  pub fn commit(&mut self, t: Transaction<NodeT>) {
    if t.committed {
      return;
    }

    self.redirect_node_vec(t.redirect_nodes);
    self.merge_nodes(t.inc_nodes);
    for (i, f) in t.mut_nodes {
      self.modify_node(i, f)
    }
    for (i, f) in t.update_nodes {
      self.update_node(i, f)
    }
    self.redirect_node_vec(t.redirect_all_nodes);
    for n in &t.dec_nodes {
      self.remove_node(*n);
    }
  }

  fn merge_nodes(&mut self, nodes: Arena<NodeT, NodeIndex>) {
    for (x, n) in &nodes {
      self.add_back_link(*x, n);
    }
    self.nodes.merge(nodes);
  }

  fn remove_node(&mut self, idx: NodeIndex) {
    let n = self.nodes.remove(idx).unwrap();
    self.remove_back_link(idx, &n);
    for (y, s) in self.back_links.remove(&idx).unwrap() {
      self.nodes.get_mut(y).unwrap().modify(s, idx, NodeIndex::empty());
    }
  }

  fn modify_node<F>(&mut self, i: NodeIndex, f: F)
  where F: FnOnce(&mut NodeT) {
    for (y, s) in self.nodes.get(i).unwrap().iter_source() {
      self.back_links.get_mut(&y).unwrap().remove(&(i, s));
    }

    f(&mut self.nodes.get_mut(i).unwrap());

    for (y, s) in self.nodes.get(i).unwrap().iter_source() {
      self.back_links.get_mut(&y).unwrap().insert((i, s));
    }
  }

  fn update_node<F>(&mut self, i: NodeIndex, f: F)
  where F: FnOnce(NodeT) -> NodeT {
    for (y, s) in self.nodes.get(i).unwrap().iter_source() {
      self.back_links.get_mut(&y).unwrap().remove(&(i, s));
    }

    self.nodes.update_with(i, |x| f(x));

    for (y, s) in self.nodes.get(i).unwrap().iter_source() {
      self.back_links.get_mut(&y).unwrap().insert((i, s));
    }
  }

  fn redirect_node(&mut self, old_node: NodeIndex, new_node: NodeIndex) {
    let old_link = self.back_links.remove(&old_node).unwrap();
    self.back_links.insert(old_node, HashSet::new());

    let new_link = self.back_links.entry(new_node).or_insert(HashSet::new());
    for (y, s) in old_link {
      new_link.insert((y, s));
      self.nodes.get_mut(y).unwrap().modify(s, old_node, new_node);
    }
  }

  fn redirect_node_vec(&mut self, replacements: Vec<(NodeIndex, NodeIndex)>) {
    let mut fa = HashMap::new();

    for (old, new) in &replacements {
      fa.entry(*old).or_insert(*old);
      fa.entry(*new).or_insert(*new);
    }

    for (old, new) in &replacements {
      let mut x = *new;
      while fa[&x] != x {
        x = fa[&x];
      }
      assert!(x != *old, "Loop redirection detected!");
      *fa.get_mut(old).unwrap() = x;
    }

    for (old, new) in &replacements {
      let mut x = *new;
      let mut y = fa[&x];
      while x != y {
        x = y;
        y = fa[&y];
      }

      self.redirect_node(*old, x);

      x = *new;
      while fa[&x] != y {
        let z = fa[&x];
        *fa.get_mut(&x).unwrap() = y;
        x = z;
      }
    }
  }

  fn add_back_link(&mut self, x: NodeIndex, n: &NodeT) {
    self.back_links.entry(x).or_insert(HashSet::new());
    for (y, s) in n.iter_source() {
      self.back_links.entry(y).or_insert(HashSet::new()).insert((x, s));
    }
  }

  fn remove_back_link(&mut self, x: NodeIndex, n: &NodeT) {
    for (y, s) in n.iter_source() {
      self.back_links.get_mut(&y).unwrap().remove(&(x, s));
    }
  }
}

pub type Iter<'a, NDataT> = hash_map::Iter<'a, NodeIndex, NDataT>;

impl<T: NodeEnum> IntoIterator for Graph<T> {
  type IntoIter = hash_map::IntoIter<NodeIndex, T>;
  type Item = (NodeIndex, T);

  fn into_iter(self) -> Self::IntoIter { self.nodes.into_iter() }
}

pub struct Transaction<'a, NodeT: NodeEnum> {
  committed: bool,
  ctx_id: Uuid,
  alloc_nodes: HashSet<NodeIndex>,
  inc_nodes: Arena<NodeT, NodeIndex>,
  dec_nodes: Vec<NodeIndex>,
  mut_nodes: Vec<(NodeIndex, Box<dyn FnOnce(&mut NodeT) + 'a>)>,
  update_nodes: Vec<(NodeIndex, Box<dyn FnOnce(NodeT) -> NodeT + 'a>)>,
  redirect_all_nodes: Vec<(NodeIndex, NodeIndex)>,
  redirect_nodes: Vec<(NodeIndex, NodeIndex)>,
}

impl<'a, NodeT: NodeEnum> Transaction<'a, NodeT> {
  pub fn new(context: &Context) -> Self {
    let node_dist = Arc::clone(&context.node_dist);
    Transaction {
      committed: false,
      ctx_id: context.id,
      alloc_nodes: HashSet::new(),
      inc_nodes: Arena::new(node_dist),
      dec_nodes: Vec::new(),
      mut_nodes: Vec::new(),
      update_nodes: Vec::new(),
      redirect_all_nodes: Vec::new(),
      redirect_nodes: Vec::new(),
    }
  }

  pub fn alloc_node(&mut self) -> NodeIndex {
    let idx = self.inc_nodes.alloc();
    self.alloc_nodes.insert(idx);
    idx
  }

  pub fn fill_back_node(&mut self, idx: NodeIndex, data: NodeT) {
    self.inc_nodes.fill_back(idx, data);
  }

  pub fn new_node(&mut self, data: NodeT) -> NodeIndex { self.inc_nodes.insert(data) }

  pub fn remove_node(&mut self, node: NodeIndex) {
    if self.inc_nodes.remove(node).is_none() {
      if !self.alloc_nodes.remove(&node) {
        self.dec_nodes.push(node);
      }
    }
  }

  pub fn mut_node<F>(&mut self, node: NodeIndex, func: F)
  where F: FnOnce(&mut NodeT) + 'a {
    if self.inc_nodes.contains(node) {
      func(&mut self.inc_nodes.get_mut(node).unwrap());
    } else {
      self.mut_nodes.push((node, Box::new(func)));
    }
  }

  pub fn update_node<F>(&mut self, node: NodeIndex, func: F)
  where F: FnOnce(NodeT) -> NodeT + 'a {
    if self.inc_nodes.contains(node) {
      self.inc_nodes.update_with(node, |x| func(x));
    } else {
      self.update_nodes.push((node, Box::new(func)));
    }
  }

  pub fn redirect_all_node(&mut self, old_node: NodeIndex, new_node: NodeIndex) {
    self.redirect_all_nodes.push((old_node, new_node));
  }

  pub fn redirect_node(&mut self, old_node: NodeIndex, new_node: NodeIndex) {
    self.redirect_nodes.push((old_node, new_node));
  }

  pub fn merge_graph(&mut self, graph: Graph<NodeT>) {
    let graph_ctx_id = graph.ctx_id;
    for (i, n) in graph.into_iter() {
      if self.ctx_id != graph_ctx_id {
        self.new_node(n);
      } else {
        self.fill_back_node(i, n);
      }
    }
  }

  pub fn giveup(&mut self) { self.committed = true; }
}

#[derive(Debug)]
pub struct Context {
  id: Uuid,
  node_dist: Arc<IdDistributer>,
}
impl Context {
  pub fn new() -> Context {
    Context {
      id: Uuid::new_v4(),
      node_dist: Arc::new(IdDistributer::new()),
    }
  }
}
impl Clone for Context {
  fn clone(&self) -> Self {
    Context {
      id: self.id,
      node_dist: Arc::clone(&self.node_dist),
    }
  }
}

pub trait SourceIterator<T: TypedNode>:
  Iterator<Item = (NodeIndex, Self::Source)>
{
  type Source: Copy + Clone + Eq + PartialEq + Debug + Hash;
  fn new(node: &T) -> Self;
}
pub trait TypedNode: Sized {
  type Source: Copy + Clone + Eq + PartialEq + Debug + Hash;
  type Iter: SourceIterator<Self, Source = Self::Source>;
  fn iter_source(&self) -> Self::Iter;
  fn modify(&mut self, source: Self::Source, old_idx: NodeIndex, new_idx: NodeIndex);
}

pub trait NodeEnum {
  type SourceEnum: Copy + Clone + Eq + PartialEq + Debug + Hash;
  fn iter_source(&self) -> Box<dyn Iterator<Item = (NodeIndex, Self::SourceEnum)>>;
  fn modify(&mut self, source: Self::SourceEnum, old_idx: NodeIndex, new_idx: NodeIndex);
}

pub trait IndexEnum {
  fn modify(&mut self, new_idx: NodeIndex);
  fn index(&self) -> NodeIndex;
}

pub struct NIEWrap<T: IndexEnum> {
  pub value: T,
}
