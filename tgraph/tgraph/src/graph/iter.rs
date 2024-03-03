use std::collections::hash_set;
use std::iter::Iterator;

use super::*;

pub struct EdgeIterator<'a: 'b, 'b, EDataT> {
  pub edges: &'a Arena<Edge<EDataT>, EdgeIndex>,
  pub iter: hash_set::Iter<'b, EdgeIndex>,
}
impl<'a: 'b, 'b, EDataT> Iterator for EdgeIterator<'a, 'b, EDataT> {
  type Item = (EdgeIndex, NodeIndex);

  fn next(&mut self) -> Option<Self::Item> {
    let e = self.iter.next();
    e.and_then(|idx| Some((*idx, self.edges.get(*idx).unwrap().from)))
  }
}
