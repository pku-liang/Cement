use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

pub type FxHasher = rustc_hash::FxHasher;
pub type FxHasherBuilder = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;
pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, FxHasherBuilder>;
pub type FxHashSet<K> = std::collections::HashSet<K, FxHasherBuilder>;
pub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxHasherBuilder>;

#[derive(Debug)]
pub struct FxMapWithUniqueId<V> {
  indexmap: FxIndexMap<usize, V>,
  next_id: usize,
}

impl<V> Default for FxMapWithUniqueId<V> {
  fn default() -> Self {
    Self {
      indexmap: Default::default(),
      next_id: Default::default(),
    }
  }
}

impl<V> Deref for FxMapWithUniqueId<V> {
  type Target = FxIndexMap<usize, V>;

  fn deref(&self) -> &Self::Target { &self.indexmap }
}

impl<V> DerefMut for FxMapWithUniqueId<V> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.indexmap }
}

impl<V> FxMapWithUniqueId<V>
where V: PartialEq + Debug + super::Id
{
  pub fn get_map(&self) -> &FxIndexMap<usize, V> { &self.indexmap }

  pub fn get_map_mut(&mut self) -> &mut FxIndexMap<usize, V> { &mut self.indexmap }

  pub fn insert_with_id<'a, 't: 'a>(&'t mut self, mut value: V) -> (usize, &'a V) {
    let cur_id = self.next_id;
    self.next_id += 1;

    value.set_id(cur_id);

    let option = self.indexmap.insert(cur_id, value);
    assert_eq!(option, None);

    (cur_id, self.indexmap.get(&cur_id).unwrap())
  }
}
