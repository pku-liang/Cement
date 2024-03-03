use std::collections::hash_map::{self, Iter, IterMut};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub trait ArenaIndex: Hash + PartialEq + Eq + Debug + Copy + Clone {
  fn new(id: usize) -> Self;
}

#[derive(Debug, Clone)]
pub struct Arena<T, IndexT: ArenaIndex> {
  distributer: Arc<IdDistributer>,
  container: HashMap<IndexT, T>,
}

impl<T, IndexT: ArenaIndex> Arena<T, IndexT> {
  pub fn new(distributer: Arc<IdDistributer>) -> Arena<T, IndexT> {
    Arena { distributer, container: HashMap::new() }
  }

  pub fn clear(&mut self) { self.container.clear() }

  pub fn insert(&mut self, item: T) -> IndexT {
    let idx = IndexT::new(self.alloc_id());
    self.container.insert(idx, item);
    idx
  }

  pub fn insert_with(&mut self, create: impl FnOnce(IndexT) -> T) -> IndexT {
    let idx = IndexT::new(self.alloc_id());
    self.container.insert(idx, create(idx));
    idx
  }

  pub fn alloc(&mut self) -> IndexT { IndexT::new(self.alloc_id()) }

  pub fn fill_back(&mut self, i: IndexT, item: T) {
    assert!(
      self.container.insert(i, item).is_none(),
      "tgraph::arena::fill_back: filled an illegal index!"
    )
  }

  pub fn remove(&mut self, i: IndexT) -> Option<T> { self.container.remove(&i) }

  pub fn contains(&self, i: IndexT) -> bool { self.container.contains_key(&i) }

  pub fn get(&self, i: IndexT) -> Option<&T> { self.container.get(&i) }

  pub fn get_mut(&mut self, i: IndexT) -> Option<&mut T> { self.container.get_mut(&i) }

  pub fn update_with<F>(&mut self, i: IndexT, f: F)
  where F: FnOnce(T) -> T {
    let x = self.container.remove(&i).unwrap();
    self.container.insert(i, f(x));
  }

  pub fn len(&self) -> usize { self.container.len() }

  pub fn is_empty(&self) -> bool { self.container.is_empty() }

  pub fn merge(&mut self, other: Arena<T, IndexT>) {
    for (idx, value) in other {
      self.fill_back(idx, value);
    }
  }

  pub fn iter(&self) -> Iter<'_, IndexT, T> { self.container.iter() }

  pub fn iter_mut(&mut self) -> IterMut<'_, IndexT, T> { self.container.iter_mut() }

  fn alloc_id(&self) -> usize { self.distributer.alloc() }
}

impl<T, IndexT: ArenaIndex> IntoIterator for Arena<T, IndexT> {
  type IntoIter = hash_map::IntoIter<IndexT, T>;
  type Item = (IndexT, T);

  fn into_iter(self) -> Self::IntoIter { self.container.into_iter() }
}
impl<'a, T, IndexT: ArenaIndex> IntoIterator for &'a Arena<T, IndexT> {
  type IntoIter = Iter<'a, IndexT, T>;
  type Item = (&'a IndexT, &'a T);

  fn into_iter(self) -> Self::IntoIter { self.container.iter() }
}
impl<'a, T, IndexT: ArenaIndex> IntoIterator for &'a mut Arena<T, IndexT> {
  type IntoIter = IterMut<'a, IndexT, T>;
  type Item = (&'a IndexT, &'a mut T);

  fn into_iter(self) -> Self::IntoIter { self.container.iter_mut() }
}

impl<T, IndexT: ArenaIndex> std::ops::Index<IndexT> for Arena<T, IndexT> {
  type Output = T;

  fn index(&self, index: IndexT) -> &Self::Output { self.get(index).unwrap() }
}
impl<T, IndexT: ArenaIndex> std::ops::IndexMut<IndexT> for Arena<T, IndexT> {
  fn index_mut(&mut self, index: IndexT) -> &mut Self::Output {
    self.get_mut(index).unwrap()
  }
}

#[derive(Debug)]
pub struct IdDistributer {
  cnt: AtomicUsize,
}

impl IdDistributer {
  pub fn new() -> IdDistributer { IdDistributer { cnt: AtomicUsize::new(0) } }

  pub fn alloc(&self) -> usize {
    let c = self.cnt.fetch_add(1, Ordering::Relaxed);
    c + 1
  }
}
