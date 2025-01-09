//! Union-Find set with distance.

use std::{collections::HashMap, fmt::Debug, hash::Hash};

/// A trait for selecting which element is parent in the union-find set.
pub trait UFOrd {
  fn uf_lt(&self, other: &Self) -> bool;
}

/// A hashmap-based union-find set.
#[derive(Debug, Clone)]
pub struct WeightedUnionFind<T: Eq + Hash + UFOrd + Clone + Debug> {
  /// keys are elements; values are their parent and distance.
  parent: HashMap<T, (T, u32)>,
  /// keys are roots; values are their descendants.
  decendants: HashMap<T, Vec<T>>,
}

impl<T: Eq + Hash + UFOrd + Clone + Debug> WeightedUnionFind<T> {
  pub fn new() -> Self {
    WeightedUnionFind {
      parent: HashMap::new(),
      decendants: HashMap::new(),
    }
  }

  /// Dump the union-find set.
  pub fn dump(&mut self) -> String {
    let mut res = String::new();

    let decendants = self.decendants.clone();
    for (root, decendants) in decendants {
      res.push_str(&format!("{:?}:\n", root));
      for decendant in decendants.iter() {
        let (parent, weight) = self.find(decendant.clone());
        assert_eq!(parent, root);
        res.push_str(&format!("\t{:?} ({})\n", decendant, weight));
      }
    }
    res

  }

  /// Get all roots.
  pub fn roots(&self) -> Vec<T> {
    self.decendants.keys().cloned().collect()
  }

  /// Check if an element exists in the union-find set.
  pub fn exists(&self, elem: &T) -> bool {
    self.parent.contains_key(elem)
  }

  /// Add an element to the union-find set.
  pub fn add_elem(&mut self, elem: T) -> (T, u32) {
    if !self.exists(&elem) {
      self.parent.insert(elem.clone(), (elem.clone(), 0));
      self.decendants.insert(elem.clone(), vec![elem.clone()]);
    }
    self.find(elem)
  }

  /// Find the root of an element.  
  pub fn find(&mut self, elem: T) -> (T, u32) {
    let (parent, weight) = self.parent[&elem].clone();
    if parent == elem {
      return (parent, weight);
    }
    let (new_parent, upper_weight) = self.find(parent);
    self
      .parent
      .insert(elem, (new_parent.clone(), upper_weight + weight));
    (new_parent, upper_weight + weight)
  }

  /// Union two elements with a distance delta.
  pub fn union(&mut self, elem1: T, elem2: T, delta: u32) -> (T, u32) {
    // elem1 = root1 + weight1
    let (root1, weight1) = self.find(elem1);
    // elem2 = root2 + weight2
    let (root2, weight2) = self.find(elem2);
    // root1 + weight1 + delta = root2 + weight2
    if root1 == root2 {
      assert!(weight1 + delta == weight2);
      return (root1, weight1);
    } else {
      // if root1 < root2, that is weight1 + delta > weight2
      let new_root = if weight1 + delta > weight2
        || (weight1 + delta == weight2 && root1.uf_lt(&root2))
      {
        root1.clone()
      } else {
        root2.clone()
      };

      let (small_root, delta_weight, total_weight) = if root1 == new_root {
        // root2 = root1 + weight1 + delta - weight2
        (root2, weight1 + delta - weight2, weight1 + delta)
      } else {
        // root1 = root2 + weight2 - weight1 - delta
        (root1, weight2 - weight1 - delta, weight2)
      };

      self
        .parent
        .insert(small_root.clone(), (new_root.clone(), delta_weight));
      let mut decendants_of_small_root =
        self.decendants.remove(&small_root).unwrap();
      self
        .decendants
        .get_mut(&new_root)
        .unwrap()
        .append(&mut decendants_of_small_root);
      (new_root, total_weight)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  struct Tuple(String, u32);

  impl ToString for Tuple {
    fn to_string(&self) -> String {
      format!("{}+{}", self.0, self.1)
    }
  }

  impl UFOrd for Tuple {
    fn uf_lt(&self, other: &Self) -> bool {
      if self.1 == other.1 {
        self.0 < other.0
      } else {
        self.1 < other.1
      }
    }
  }

  #[test]
  fn test_weighted_union_find() {
    let mut uf = WeightedUnionFind::new();
    uf.add_elem(Tuple("a".to_string(), 0));
    uf.add_elem(Tuple("a".to_string(), 1));
    uf.add_elem(Tuple("b".to_string(), 0));
    uf.add_elem(Tuple("c".to_string(), 1));
    uf.add_elem(Tuple("d".to_string(), 2));
    uf.add_elem(Tuple("e".to_string(), 0));

    uf.union(Tuple("a".to_string(), 0), Tuple("a".to_string(), 1), 1);
    uf.union(Tuple("b".to_string(), 0), Tuple("a".to_string(), 1), 0);
    uf.union(Tuple("c".to_string(), 1), Tuple("a".to_string(), 1), 2);

    uf.union(Tuple("d".to_string(), 2), Tuple("e".to_string(), 0), 1);

    assert!(uf.exists(&Tuple("a".to_string(), 0)));

    println!(
      "{:?} canonicalized: {:?}",
      Tuple("a".to_string(), 0),
      uf.find(Tuple("a".to_string(), 0))
    );
    println!(
      "{:?} canonicalized: {:?}",
      Tuple("a".to_string(), 1),
      uf.find(Tuple("a".to_string(), 1))
    );
    println!(
      "{:?} canonicalized: {:?}",
      Tuple("b".to_string(), 0),
      uf.find(Tuple("b".to_string(), 0))
    );
    println!(
      "{:?} canonicalized: {:?}",
      Tuple("c".to_string(), 1),
      uf.find(Tuple("c".to_string(), 1))
    );
    println!(
      "{:?} canonicalized: {:?}",
      Tuple("d".to_string(), 2),
      uf.find(Tuple("d".to_string(), 2))
    );
    println!(
      "{:?} canonicalized: {:?}",
      Tuple("e".to_string(), 0),
      uf.find(Tuple("e".to_string(), 0))
    );
  }
}
