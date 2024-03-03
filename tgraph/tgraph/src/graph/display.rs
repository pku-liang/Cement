use std::fmt::Display;

use super::*;

impl Display for NodeIndex {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "NodeIndex({})", self.0)?;
    std::fmt::Result::Ok(())
  }
}
impl Display for EdgeIndex {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "EdgeIndex({})", self.0)?;
    std::fmt::Result::Ok(())
  }
}

impl<NDataT: Display> Display for Node<NDataT> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Node {{ idx: {}, data: {}, in_edges: [", self.idx.0, self.data)?;
    for i in &self.in_edges {
      write!(f, "{}, ", i.0)?;
    }
    write!(f, "], out_edges: [")?;
    for i in &self.out_edges {
      write!(f, "{}, ", i.0)?;
    }
    write!(f, "]}}")?;
    std::fmt::Result::Ok(())
  }
}

impl<EDataT: Display> Display for Edge<EDataT> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Edge{{ idx: {}, data: {}, from: {}, to: {} }}",
      &self.idx.0, &self.data, &self.from.0, &self.to.0
    )?;
    std::fmt::Result::Ok(())
  }
}

impl<NDataT: Display, EDataT: Display> Display for Graph<NDataT, EDataT> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Graph {{\n  nodes: [\n")?;
    for (_, n) in &self.nodes {
      write!(f, "     {},\n", n)?;
    }
    write!(f, "  ],\n  edges: [\n")?;
    for (_, n) in &self.edges {
      write!(f, "     {},\n", n)?;
    }
    write!(f, "  ]\n}}")?;
    std::fmt::Result::Ok(())
  }
}
