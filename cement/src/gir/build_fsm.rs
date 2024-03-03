use std::collections::HashSet;

use itertools::Itertools;
use tgraph::typed_graph::{Context, Graph, NodeIndex, Transaction};

use super::component::*;
use super::construction::*;
use crate::preclude::hashset_addone;
use crate::utils::hashset_only_element;

pub(super) fn traverse_ast<'a>(
  ctx: &Context, root: NodeIndex, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let node = graph.get_node(root).unwrap();
  match node {
    Component::AstStep(step) => make_ast_step(ctx, step, region),
    Component::AstSeq(seq) => make_ast_seq(ctx, seq, graph, region),
    Component::AstPar(par) => make_ast_par(ctx, par, graph, region),
    Component::AstIf(ast_if) => make_ast_if(ctx, ast_if, graph, region),
    Component::AstIfElse(ast_if_else) => {
      make_ast_if_else(ctx, ast_if_else, graph, region)
    },
    Component::AstFor(ast_for) => make_ast_for(ctx, ast_for, graph, region),
    Component::AstWhile(ast_while) => make_ast_while(ctx, ast_while, graph, region),
    _ => panic!("Unknown node in AST!"),
  }
}

fn make_ast_step<'a>(
  ctx: &Context, step: &AstStep, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);

  let (idle_idx, mut idle_state) = empty_state(&mut fsm_trans);
  let (work_idx, mut work_state) = new_state(&mut fsm_trans, &step.events);
  let leaf = fsm_trans.new_node(Component::LeafNode(LeafNode { state: work_idx }));

  let true_lit = new_true(&mut fsm_trans, region, step.location);

  // Idle to Work
  new_simple_transistion(
    &mut fsm_trans,
    true_lit,
    &HashSet::new(),
    (idle_idx, &mut idle_state),
    (work_idx, &mut work_state),
  );
  if step.waits.is_empty() {
    // Work to Idle
    new_simple_transistion(
      &mut fsm_trans,
      true_lit,
      &HashSet::new(),
      (work_idx, &mut work_state),
      (idle_idx, &mut idle_state),
    );
  } else {
    let (wait_idx, mut wait_state) = empty_state(&mut fsm_trans);

    let cond = new_reduce_and(&mut fsm_trans, &step.waits, region, 1, step.location);
    new_simple_transistion(
      &mut fsm_trans,
      cond,
      &HashSet::new(),
      (work_idx, &mut work_state),
      (idle_idx, &mut idle_state),
    );
    new_simple_transistion(
      &mut fsm_trans,
      cond,
      &HashSet::new(),
      (wait_idx, &mut wait_state),
      (idle_idx, &mut idle_state),
    );

    let not_cond = new_not(&mut fsm_trans, cond, region, 1, step.location);
    new_simple_transistion(
      &mut fsm_trans,
      not_cond,
      &HashSet::new(),
      (work_idx, &mut work_state),
      (wait_idx, &mut wait_state),
    );
    new_self_transistion(
      &mut fsm_trans,
      not_cond,
      &HashSet::new(),
      (wait_idx, &mut wait_state),
    );
    fsm_trans.fill_back_node(wait_idx, Component::State(wait_state));
  }
  fsm_trans.fill_back_node(idle_idx, Component::State(idle_state));
  fsm_trans.fill_back_node(work_idx, Component::State(work_state));
  fsm.commit(fsm_trans);
  // eprintln!("Step {:?}", fsm);

  (idle_idx, leaf, fsm)
}

fn make_ast_seq(
  ctx: &Context, seq: &AstSeq, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut children = Vec::new();
  for c in &seq.children {
    let (idle_idx, root_idx, fsm) = traverse_ast(ctx, *c, graph, region);
    let sorted = fsm_take_and_sort(fsm, idle_idx, root_idx);
    children.push((idle_idx, root_idx, sorted));
  }
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);

  let idle_idx = children.first().unwrap().0;
  let mut root = ExcNode {
    children: Vec::new(),
    encoding: Vec::new(),
  };
  let mut idle_state = State {
    events: HashSet::new(),
    // froms: HashSet::new(),
    // tos: HashSet::new(),
  };
  let mut last_transit = Vec::new();

  let n = children.len();
  for (cnt, (_, root_idx, sorted)) in children.into_iter().enumerate() {
    for (i, state) in sorted.non_idle_state {
      fsm_trans.fill_back_node(i, Component::State(state));
    }
    if cnt == 0 {
      idle_state.events = sorted.idle_state.events;
      // idle_state.tos = sorted.idle_state.tos;
      for (i, transit) in sorted.entry_transit {
        // eprintln!("Entry {:?}", i);
        fsm_trans.fill_back_node(i, Component::Transition(transit));
      }
    } else {
      for (_, entry) in sorted.entry_transit {
        for (_, exit) in &last_transit {
          new_merge_transition(&mut fsm_trans, exit, &entry, region, seq.location);
        }
      }
    }

    if cnt != n - 1 {
      last_transit = sorted.exit_transit;
    } else {
      // idle_state.froms = sorted.idle_state.froms;
      for (i, mut transit) in sorted.exit_transit {
        transit.to = idle_idx;
        fsm_trans.fill_back_node(i, Component::Transition(transit));
      }
    }

    for (i, transit) in sorted.other_transit {
      fsm_trans.fill_back_node(i, Component::Transition(transit));
    }

    if let Component::ExcNode(node) = sorted.state_root {
      root.children.extend(node.children.into_iter());
    } else {
      root.children.push(root_idx);
      fsm_trans.fill_back_node(root_idx, sorted.state_root);
    }
    for (i, n) in sorted.other_nodes {
      fsm_trans.fill_back_node(i, n);
    }
  }

  fsm_trans.fill_back_node(idle_idx, Component::State(idle_state));
  let root_idx = fsm_trans.new_node(Component::ExcNode(root));
  fsm.commit(fsm_trans);
  // eprintln!("{:?}", fsm);

  (idle_idx, root_idx, fsm)
}

fn make_ast_par(
  ctx: &Context, par: &AstPar, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut children = Vec::new();
  for c in &par.children {
    let (idle_idx, root_idx, fsm) = traverse_ast(ctx, *c, graph, region);
    let sorted = fsm_take_and_sort(fsm, idle_idx, root_idx);
    children.push((idle_idx, root_idx, sorted));
  }

  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);

  let (idle_idx, idle_state) = empty_state(&mut fsm_trans);
  let mut root = ParNode {
    children: Vec::new(),
    encoding: Vec::new(),
  };

  let mut exit_transits = Vec::new();

  for (_, c_root_idx, sorted) in children {
    for (i, state) in sorted.non_idle_state {
      fsm_trans.fill_back_node(i, Component::State(state));
    }

    for (i, mut transit) in sorted.entry_transit {
      transit.froms.clear();
      transit.froms.insert(idle_idx);
      // idle_state.tos.insert(i);
      fsm_trans.fill_back_node(i, Component::Transition(transit));
    }
    exit_transits.push(sorted.exit_transit);

    for (i, transit) in sorted.other_transit {
      fsm_trans.fill_back_node(i, Component::Transition(transit));
    }

    if let Component::ParNode(node) = sorted.state_root {
      root.children.extend(node.children.into_iter());
    } else {
      root.children.push(c_root_idx);
      fsm_trans.fill_back_node(c_root_idx, sorted.state_root);
    }

    for (i, n) in sorted.other_nodes {
      fsm_trans.fill_back_node(i, n);
    }
  }

  for combo in exit_transits.into_iter().multi_cartesian_product() {
    let mut acts = HashSet::new();
    let mut froms = HashSet::new();
    let mut conds = Vec::new();
    for (_, transit) in combo {
      acts.extend(transit.acts.into_iter());
      froms.extend(transit.froms.into_iter());
      conds.push(transit.cond);
    }
    let cond = new_reduce_and(&mut fsm_trans, &conds, region, 1, par.location);
    // idle_state.froms.insert(
    fsm_trans.new_node(Component::Transition(Transition {
      acts,
      cond,
      froms,
      to: idle_idx,
      event: NodeIndex::empty(),
    }));
  }

  fsm_trans.fill_back_node(idle_idx, Component::State(idle_state));
  let root_idx = fsm_trans.new_node(Component::ParNode(root));

  fsm.commit(fsm_trans);

  (idle_idx, root_idx, fsm)
}

fn make_ast_if(
  ctx: &Context, ast_if: &AstIf, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);
  let true_lit = new_true(&mut fsm_trans, region, ast_if.location);

  let (then_idle_idx, then_root_idx, then_fsm) =
    traverse_ast(ctx, ast_if.then, graph, region);
  let then_sorted = fsm_take_and_sort(then_fsm, then_idle_idx, then_root_idx);
  let mut idle_state = then_sorted.idle_state;

  for (i, state) in then_sorted.non_idle_state {
    fsm_trans.fill_back_node(i, Component::State(state));
  }

  for (i, transit) in then_sorted.entry_transit {
    let cond =
      new_and(&mut fsm_trans, ast_if.cond, transit.cond, region, 1, ast_if.location);
    fsm_trans.fill_back_node(i, Component::Transition(Transition { cond, ..transit }));
  }
  for (i, transit) in then_sorted.exit_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }
  for (i, transit) in then_sorted.other_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  for (i, node) in then_sorted.other_nodes {
    fsm_trans.fill_back_node(i, node);
  }

  let not_cond = new_not(&mut fsm_trans, ast_if.cond, region, 1, ast_if.location);
  let (empty_idx, mut empty_state) = new_state(&mut fsm_trans, &HashSet::new());
  new_simple_transistion(
    &mut fsm_trans,
    not_cond,
    &HashSet::new(),
    (then_idle_idx, &mut idle_state),
    (empty_idx, &mut empty_state),
  );
  new_simple_transistion(
    &mut fsm_trans,
    true_lit,
    &HashSet::new(),
    (empty_idx, &mut empty_state),
    (then_idle_idx, &mut idle_state),
  );
  fsm_trans.fill_back_node(empty_idx, Component::State(empty_state));

  fsm_trans.fill_back_node(then_root_idx, then_sorted.state_root);
  fsm_trans.fill_back_node(then_idle_idx, Component::State(idle_state));
  fsm.commit(fsm_trans);
  (then_idle_idx, then_root_idx, fsm)
}

fn make_ast_if_else(
  ctx: &Context, ast_if_else: &AstIfElse, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);
  let (idle_idx, idle_state) = empty_state(&mut fsm_trans);
  let mut root = ExcNode {
    children: Vec::new(),
    encoding: Vec::new(),
  };

  let (then_idle_idx, then_root_idx, then_fsm) =
    traverse_ast(ctx, ast_if_else.then, graph, region);
  let then_sorted = fsm_take_and_sort(then_fsm, then_idle_idx, then_root_idx);
  let (alt_idle_idx, alt_root_idx, alt_fsm) =
    traverse_ast(ctx, ast_if_else.alt, graph, region);
  let alt_sorted = fsm_take_and_sort(alt_fsm, alt_idle_idx, alt_root_idx);

  let not_cond =
    new_not(&mut fsm_trans, ast_if_else.cond, region, 1, ast_if_else.location);

  for (i, state) in then_sorted.non_idle_state {
    fsm_trans.fill_back_node(i, Component::State(state));
  }
  for (i, state) in alt_sorted.non_idle_state {
    fsm_trans.fill_back_node(i, Component::State(state));
  }

  for (i, mut transit) in then_sorted.entry_transit {
    transit.cond = new_and(
      &mut fsm_trans,
      ast_if_else.cond,
      transit.cond,
      region,
      1,
      ast_if_else.location,
    );
    transit.froms.clear();
    transit.froms.insert(idle_idx);
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }
  for (i, mut transit) in alt_sorted.entry_transit {
    transit.cond =
      new_and(&mut fsm_trans, not_cond, transit.cond, region, 1, ast_if_else.location);
    transit.froms.clear();
    transit.froms.insert(idle_idx);
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  for (i, mut transit) in then_sorted.exit_transit {
    transit.to = idle_idx;
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }
  for (i, mut transit) in alt_sorted.exit_transit {
    transit.to = idle_idx;
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  for (i, transit) in then_sorted.other_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }
  for (i, transit) in alt_sorted.other_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  if let Component::ExcNode(node) = then_sorted.state_root {
    root.children.extend(node.children.into_iter());
  } else {
    root.children.push(then_root_idx);
    fsm_trans.fill_back_node(then_root_idx, then_sorted.state_root);
  }
  if let Component::ExcNode(node) = alt_sorted.state_root {
    root.children.extend(node.children.into_iter());
  } else {
    root.children.push(alt_root_idx);
    fsm_trans.fill_back_node(alt_root_idx, alt_sorted.state_root);
  }

  for (i, node) in then_sorted.other_nodes {
    fsm_trans.fill_back_node(i, node);
  }
  for (i, node) in alt_sorted.other_nodes {
    fsm_trans.fill_back_node(i, node);
  }

  fsm_trans.fill_back_node(idle_idx, Component::State(idle_state));
  let root_idx = fsm_trans.new_node(Component::ExcNode(root));
  fsm.commit(fsm_trans);

  (idle_idx, root_idx, fsm)
}
fn make_ast_for(
  ctx: &Context, ast_for: &AstFor, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);

  let (idle_idx, root_idx, body_fsm) = traverse_ast(ctx, ast_for.body, graph, region);
  let sorted = fsm_take_and_sort(body_fsm, idle_idx, root_idx);

  for (i, state) in sorted.non_idle_state {
    fsm_trans.fill_back_node(i, Component::State(state));
  }

  let start = if ast_for.start.is_empty() {
    new_literal(
      &mut fsm_trans,
      ast_for.c_start,
      ast_for.loop_var_width,
      region,
      ast_for.location,
    )
  } else {
    ast_for.start
  };
  let end = if ast_for.end.is_empty() {
    new_literal(
      &mut fsm_trans,
      ast_for.c_end,
      ast_for.loop_var_width,
      region,
      ast_for.location,
    )
  } else {
    ast_for.end
  };
  let step = if ast_for.step.is_empty() {
    new_literal(
      &mut fsm_trans,
      ast_for.c_step,
      ast_for.loop_var_width,
      region,
      ast_for.location,
    )
  } else {
    ast_for.end
  };
  let assign_start = new_assign(&mut fsm_trans, ast_for.loop_var_wr, start, region);
  // new_reset_event(&mut fsm_trans, vec![assign_start]);
  // TODO: find the reset signal

  let increase = new_add(
    &mut fsm_trans,
    ast_for.loop_var_rd,
    step,
    region,
    ast_for.loop_var_width,
    ast_for.location,
  );
  let assign_inc = new_assign(&mut fsm_trans, ast_for.loop_var_wr, increase, region);

  // TODO: compare when step is not 1
  let end_cond = new_ge(&mut fsm_trans, increase, end, region, 1, ast_for.location);
  let not_end_cond = new_not(&mut fsm_trans, end_cond, region, 1, ast_for.location);

  // TODO: prevent empty

  // TODO: solve dependency

  for (i, e1) in sorted.exit_transit {
    let new_cond =
      new_and(&mut fsm_trans, end_cond, e1.cond, region, 1, ast_for.location);
    let inc_acts = hashset_addone(e1.acts.clone(), assign_inc);
    fsm_trans.fill_back_node(
      i,
      Component::Transition(Transition {
        acts: e1.acts,
        cond: new_cond,
        froms: e1.froms.clone(),
        to: e1.to,
        event: NodeIndex::empty(),
      }),
    );
    for (_, e2) in &sorted.entry_transit {
      let new_cond =
        new_and(&mut fsm_trans, not_end_cond, e1.cond, region, 1, ast_for.location);
      let new_cond =
        new_and(&mut fsm_trans, new_cond, e2.cond, region, 1, ast_for.location);
      fsm_trans.new_node(Component::Transition(Transition {
        acts: inc_acts.clone(),
        cond: new_cond,
        froms: e1.froms.clone(),
        to: e2.to,
        event: NodeIndex::empty(),
      }));
    }
  }
  for (i, transit) in sorted.entry_transit {
    fsm_trans.fill_back_node(
      i,
      Component::Transition(Transition {
        acts: hashset_addone(transit.acts, assign_start),
        ..transit
      }),
    );
  }

  for (i, transit) in sorted.other_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  fsm_trans.fill_back_node(root_idx, sorted.state_root);

  for (i, node) in sorted.other_nodes {
    fsm_trans.fill_back_node(i, node);
  }
  fsm_trans.fill_back_node(idle_idx, Component::State(sorted.idle_state));
  fsm.commit(fsm_trans);

  (idle_idx, root_idx, fsm)
}
fn make_ast_while(
  ctx: &Context, ast_while: &AstWhile, graph: &Graph<Component>, region: NodeIndex,
) -> (NodeIndex, NodeIndex, Graph<Component>) {
  let mut fsm = Graph::new(ctx);
  let mut fsm_trans = Transaction::new(ctx);

  let (idle_idx, root_idx, body_fsm) = traverse_ast(ctx, ast_while.body, graph, region);
  let sorted = fsm_take_and_sort(body_fsm, idle_idx, root_idx);
  let mut idle_state = sorted.idle_state;
  let true_lit = new_true(&mut fsm_trans, region, ast_while.location);

  for (i, state) in sorted.non_idle_state {
    fsm_trans.fill_back_node(i, Component::State(state));
  }

  let not_cond = new_not(&mut fsm_trans, ast_while.cond, region, 1, ast_while.location);

  // TODO: prevent empty

  // TODO: solve dependency

  for (_, e1) in sorted.exit_transit {
    let new_cond =
      new_and(&mut fsm_trans, ast_while.cond, e1.cond, region, 1, ast_while.location);
    fsm_trans.new_node(Component::Transition(Transition {
      acts: e1.acts.clone(),
      cond: new_cond,
      froms: e1.froms.clone(),
      to: e1.to,
      event: NodeIndex::empty(),
    }));
    for (_, e2) in &sorted.entry_transit {
      let new_cond =
        new_and(&mut fsm_trans, not_cond, e1.cond, region, 1, ast_while.location);
      let new_cond =
        new_and(&mut fsm_trans, new_cond, e2.cond, region, 1, ast_while.location);
      fsm_trans.new_node(Component::Transition(Transition {
        acts: e1.acts.clone(),
        cond: new_cond,
        froms: e1.froms.clone(),
        to: e2.to,
        event: NodeIndex::empty(),
      }));
    }
  }

  let (empty_idx, mut empty_state) = new_state(&mut fsm_trans, &HashSet::new());
  for (i, transit) in sorted.entry_transit {
    let new_cond = new_and(
      &mut fsm_trans,
      ast_while.cond,
      transit.cond,
      region,
      1,
      ast_while.location,
    );
    fsm_trans
      .fill_back_node(i, Component::Transition(Transition { cond: new_cond, ..transit }));
    new_simple_transistion(
      &mut fsm_trans,
      not_cond,
      &HashSet::new(),
      (idle_idx, &mut idle_state),
      (empty_idx, &mut empty_state),
    );
  }
  new_simple_transistion(
    &mut fsm_trans,
    true_lit,
    &HashSet::new(),
    (empty_idx, &mut empty_state),
    (idle_idx, &mut idle_state),
  );
  fsm_trans.fill_back_node(empty_idx, Component::State(empty_state));

  for (i, transit) in sorted.other_transit {
    fsm_trans.fill_back_node(i, Component::Transition(transit));
  }

  fsm_trans.fill_back_node(root_idx, sorted.state_root);

  for (i, node) in sorted.other_nodes {
    fsm_trans.fill_back_node(i, node);
  }
  fsm_trans.fill_back_node(idle_idx, Component::State(idle_state));
  fsm.commit(fsm_trans);
  (idle_idx, root_idx, fsm)
}

// fn prevent_empty(
//     fsm_trans: &mut Transaction<'_, Component>, idle_idx: NodeIndex, idle_state: State,
//     entry_transit: Vec<(NodeIndex, Transition)>,
//     exit_transit: Vec<(NodeIndex, Transition)>,
// ) -> (State, Vec<(NodeIndex, Transition)>, Vec<(NodeIndex, Transition)>) {
// }

pub(super) struct SortedFSM {
  idle_state: State,
  non_idle_state: Vec<(NodeIndex, State)>,
  entry_transit: Vec<(NodeIndex, Transition)>,
  exit_transit: Vec<(NodeIndex, Transition)>,
  other_transit: Vec<(NodeIndex, Transition)>,
  state_root: Component,
  other_nodes: Vec<(NodeIndex, Component)>,
}
pub(super) fn fsm_take_and_sort(
  fsm: Graph<Component>, idle_idx: NodeIndex, root_idx: NodeIndex,
) -> SortedFSM {
  let mut idle_state = None;
  let mut non_idle_state = Vec::new();
  let mut entry_transit = Vec::new();
  let mut exit_transit = Vec::new();
  let mut other_transit = Vec::new();
  let mut state_root = None;
  let mut other_nodes = Vec::new();
  for (i, n) in fsm.into_iter() {
    match n {
      Component::State(state) => {
        if i == idle_idx {
          idle_state = Some(state);
        } else {
          non_idle_state.push((i, state));
        }
      },
      Component::Transition(transit) => {
        if transit.to == idle_idx {
          exit_transit.push((i, transit));
        } else if hashset_only_element(&transit.froms, &idle_idx) {
          entry_transit.push((i, transit));
        } else {
          other_transit.push((i, transit));
        }
      },
      Component::ExcNode(node) => {
        if i == root_idx {
          state_root = Some(Component::ExcNode(node))
        } else {
          other_nodes.push((i, Component::ExcNode(node)))
        }
      },
      Component::ParNode(node) => {
        if i == root_idx {
          state_root = Some(Component::ParNode(node))
        } else {
          other_nodes.push((i, Component::ParNode(node)))
        }
      },
      Component::LeafNode(node) => {
        if i == root_idx {
          state_root = Some(Component::LeafNode(node))
        } else {
          other_nodes.push((i, Component::LeafNode(node)))
        }
      },
      x => other_nodes.push((i, x)),
    }
  }
  SortedFSM {
    idle_state: idle_state.unwrap(),
    non_idle_state,
    entry_transit,
    exit_transit,
    other_transit,
    state_root: state_root.unwrap(),
    other_nodes,
  }
}
