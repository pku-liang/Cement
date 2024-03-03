#[cfg(test)]
mod tests {
  use tgraph::graph::*;

  #[test]
  fn can_compile() {
    let context = Context::new();
    let mut graph = Graph::<i64, i64>::new(&context);
    let mut trans = Transaction::new(&context);
    let n1 = trans.new_node(1);
    let n2 = trans.new_node(2);
    let e1 = trans.new_edge(-1, n1, n2);
    graph.commit(trans);
    println!("{}", graph);

    let mut trans = Transaction::new(&context);
    let n3 = trans.new_node(3);
    let n4 = trans.new_node(4);
    trans.new_edge(-2, n1, n3);
    trans.new_edge(-3, n4, n2);
    trans.new_edge(-4, n4, n3);
    trans.remove_edge(e1);
    graph.commit(trans);
    println!("{}", graph);

    let mut trans = Transaction::new(&context);
    trans.remove_node(n1);
    trans.mut_node(n2, |x| *x = *x * 5);
    graph.commit(trans);
    println!("{}", graph);

    let mut trans = Transaction::new(&context);
    let n5 = trans.new_node(5);
    trans.new_edge(-5, n2, n5);
    graph.commit(trans);

    let mut trans = Transaction::new(&context);
    trans.update_node(n5, |x| x * 3);
    graph.commit(trans);
    println!("{}", graph);

    for (e, x) in graph.iter_in(n3) {
      println!("{} {}", e, x);
    }
  }
}
