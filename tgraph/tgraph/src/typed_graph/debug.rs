use std::fmt;
use std::fmt::Debug;

use super::*;

impl<T: NodeEnum + Debug> Debug for Graph<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Graph {{\n")?;
    write!(f, "  ctx_id = {:?}", self.ctx_id)?;
    write!(f, "  nodes = [\n")?;
    for (i, n) in self.iter_nodes() {
      write!(f, "    {:?}: {:?}\n", i, n)?;
    }
    write!(f, "  ],\n")?;
    write!(f, "  back_link=[\n")?;
    for (i, s) in self.back_links.iter() {
      write!(f, "    {:?}: {:?}\n", i, s)?;
    }
    write!(f, "  ]\n")?;
    write!(f, "}}\n")?;
    Ok(())
  }
}
