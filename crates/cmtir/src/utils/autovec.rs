
#[derive(Default, Debug, Clone)]
pub struct AutoVec<T: Clone> {
  inner: Vec<Option<T>>,
}

impl<T: Clone> AutoVec<T> {
  pub fn new() -> Self {
    Self { inner: Vec::new() }
  }

  pub fn insert(&mut self, idx: usize, item: T) -> T {
    if idx >= self.inner.len() {
      self.inner.resize_with(idx + 1, Default::default);
    }
    self.inner[idx] = Some(item.clone());
    item
  }

  pub fn has(&self, idx: usize) -> bool {
    idx < self.inner.len() && self.inner[idx].is_some()
  }

  pub fn get(&self, idx: usize) -> &T {
    &self.inner[idx].as_ref().unwrap()
  }

  pub fn get_mut(&mut self, idx: usize) -> &mut T {
    self.inner[idx].as_mut().unwrap()
  }

  pub fn get_option(&self, idx: usize) -> Option<&T> {
    if idx >= self.inner.len() {
      return None;
    }
    self.inner[idx].as_ref()
  }

  pub fn get_or_default_mut(&mut self, idx: usize) -> &mut Option<T> {
    if idx >= self.inner.len() {
      self.inner.resize_with(idx + 1, Default::default);
    }
    &mut self.inner[idx]
  }
}

impl<T: Clone + Default> Into<Vec<T>> for AutoVec<T> {
  fn into(self) -> Vec<T> {
    self.inner.into_iter().flatten().collect()
  }
}

pub fn string_match(s1: &[String], s2: &[String]) -> bool {
  if s1.len() != s2.len() {
    return false;
  }
  for (s1, s2) in s1.iter().zip(s2.iter()) {
    if s1 != s2 {
      return false;
    }
  }
  true
}

impl<T: Clone> IntoIterator for AutoVec<T> {
  type Item = (usize, T);

  type IntoIter = std::vec::IntoIter<(usize, T)>;
  fn into_iter(self) -> Self::IntoIter {
    self
      .inner
      .into_iter()
      .enumerate()
      .filter_map(|(i, v)| v.map(|v| (i, v)))
      .collect::<Vec<_>>()
      .into_iter()
  }
}

pub struct AutoVecIter<'a, T: Clone> {
  inner: &'a AutoVec<T>,
  idx: usize,
}

impl<'a, T: Clone> Iterator for AutoVecIter<'a, T> {
  type Item = (usize, &'a T);
  fn next(&mut self) -> Option<Self::Item> {
    while self.idx < self.inner.inner.len() {
      let item = self.inner.inner[self.idx].as_ref();
      self.idx += 1;
      if item.is_some() {
        return Some((self.idx - 1, item.unwrap()));
      }
    }
    None
  }
}

impl<T: Clone> AutoVec<T> {
  pub fn iter(&self) -> AutoVecIter<T> {
    AutoVecIter {
      inner: self,
      idx: 0,
    }
  }
}

/// Mutable Iterator for `AutoVec`
pub struct AutoVecIterMut<'a, T: Clone> {
  inner: &'a mut AutoVec<T>,
  idx: usize,
}

impl<'a, T: Clone> Iterator for AutoVecIterMut<'a, T> {
  type Item = (usize, &'a mut T);

  fn next(&mut self) -> Option<Self::Item> {
      while self.idx < self.inner.inner.len() {
          // SAFETY: We ensure that only one mutable reference is active at a time.
          // The lifetime `'a` ties the mutable reference to the iterator's lifetime.
          if let Some(ref mut item_option) = self.inner.inner[self.idx] {
              let current_idx = self.idx;
              // It's safe to split the mutable reference here
              // because we increment `self.idx` immediately after.
              // This ensures no two mutable references exist simultaneously.
              let item = unsafe {
                  // Obtain a raw pointer to the item
                  let ptr = item_option as *mut T;
                  // Convert the raw pointer back to a mutable reference
                  &mut *ptr
              };
              self.idx += 1;
              return Some((current_idx, item));
          }
          self.idx += 1;
      }
      None
  }
}

impl<T: Clone> AutoVec<T> {
  pub fn iter_mut(&mut self) -> AutoVecIterMut<T> {
    AutoVecIterMut {
      inner: self,
      idx: 0,
    }
  }
}
