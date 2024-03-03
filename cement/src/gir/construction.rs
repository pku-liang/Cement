use std::collections::HashSet;
use std::panic::Location;

use irony_cmt::RegionId;
use tgraph::typed_graph::*;

use super::component::*;
use crate::utils::*;

pub(super) fn empty_state(trans: &mut Transaction<'_, Component>) -> (NodeIndex, State) {
  let idx = trans.alloc_node();
  let data = State {
    events: HashSet::new(),
    // froms: HashSet::new(),
    // tos: HashSet::new(),
  };
  (idx, data)
}

pub(super) fn new_state(
  trans: &mut Transaction<'_, Component>, events: &HashSet<NodeIndex>,
) -> (NodeIndex, State) {
  let idx = trans.alloc_node();
  let data = State {
    events: events.clone(),
    // froms: HashSet::new(),
    // tos: HashSet::new(),
  };
  (idx, data)
}

pub(super) fn new_simple_transistion(
  trans: &mut Transaction<'_, Component>, cond: NodeIndex, acts: &HashSet<NodeIndex>,
  from: (NodeIndex, &mut State), to: (NodeIndex, &mut State),
) -> NodeIndex {
  let mut transit = Transition {
    acts: acts.clone(),
    cond,
    froms: HashSet::new(),
    to: to.0,
    event: NodeIndex::empty(),
  };
  let idx = trans.alloc_node();
  transit.froms.insert(from.0);
  // from.1.tos.insert(idx);
  // to.1.froms.insert(idx);

  trans.fill_back_node(idx, Component::Transition(transit));
  idx
}
pub(super) fn new_self_transistion(
  trans: &mut Transaction<'_, Component>, cond: NodeIndex, acts: &HashSet<NodeIndex>,
  state: (NodeIndex, &mut State),
) -> NodeIndex {
  let mut transit = Transition {
    acts: acts.clone(),
    cond,
    froms: HashSet::new(),
    to: state.0,
    event: NodeIndex::empty(),
  };
  transit.froms.insert(state.0);
  let idx = trans.new_node(Component::Transition(transit));
  // state.1.tos.insert(idx);
  // state.1.froms.insert(idx);

  idx
}

pub(super) fn new_merge_transition(
  trans: &mut Transaction<'_, Component>, e1: &Transition, e2: &Transition,
  region: NodeIndex, location: Location<'static>,
) -> NodeIndex {
  let cond = new_and(trans, e1.cond, e2.cond, region, 1, location);
  let mut transit = Transition {
    acts: hashset_merge(&e1.acts, &e2.acts),
    cond,
    froms: e1.froms.clone(),
    to: e2.to,
    event: NodeIndex::empty(),
  };

  trans.new_node(Component::Transition(transit))
}

pub(super) fn new_wire(
  trans: &mut Transaction<'_, Component>, width: usize, region: NodeIndex,
  location: Location<'static>,
) -> NodeIndex {
  // eprintln!("Made new wire!");
  trans.new_node(Component::Wire(Wire { width, region, entity_id: None, location }))
}
pub(super) fn new_state_reg(
  trans: &mut Transaction<'_, Component>, width: usize, region: NodeIndex,
  location: Location<'static>,
) -> NodeIndex {
  let wire_in = Vec::from_iter((0..width).map(|_| new_wire(trans, 1, region, location)));
  let wire_out = Vec::from_iter((0..width).map(|_| new_wire(trans, 1, region, location)));
  trans.new_node(Component::StateReg(StateReg {
    wire_in,
    wire_out,
    width,
    region,
    location,
  }))
}

pub(super) fn new_assign(
  trans: &mut Transaction<'_, Component>, lhs: NodeIndex, rhs: NodeIndex,
  region: NodeIndex,
) -> NodeIndex {
  trans.new_node(Component::Assign(Assign { lhs, rhs, region }))
}
// pub(super) fn new_reset_event(
//     trans: &mut Transaction<'_, Component>, assigns: Vec<NodeIndex>,
// ) -> NodeIndex {
//     trans.new_node(Component::ResetEvent(ResetEvent {
//         assigns: assigns.into_iter().collect(),
//         entity_id: None,
//     }))
// }

// Construct Expr, not declaring wires

pub(super) fn new_not(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, region: NodeIndex, width: usize,
  location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::UnaryOp(UnaryOp {
    operand: x,
    region,
    width,
    ty: UnaryOpType::Not,
    location,
  }))
}
pub(super) fn new_neg(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, region: NodeIndex, width: usize,
  location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::UnaryOp(UnaryOp {
    operand: x,
    ty: UnaryOpType::Neg,
    region,
    width,
    location,
  }))
}
pub(super) fn new_add(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Add,
    region,
    width,
    location,
  }))
}
// pub(super) fn new_sub(
//   trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: RegionId,
//   width: usize,
// ) -> NodeIndex {
//   trans.new_node(Component::BinaryOp(BinaryOp {
//     operand1: x,
//     operand2: y,
//     ty: BinaryOpType::Sub,
//     region,
//     width,
//   }))
// }
pub(super) fn new_and(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  // TODO: find a better way do deal with true
  if x.is_empty() {
    return y;
  }
  if y.is_empty() {
    return x;
  }
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::And,
    region,
    width,
    location,
  }))
}

pub(super) fn new_or(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Or,
    region,
    width,
    location,
  }))
}
pub(super) fn new_xor(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Xor,
    region,
    width,
    location,
  }))
}
pub(super) fn new_eq(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Eq,
    region,
    width,
    location,
  }))
}
pub(super) fn new_neq(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Neq,
    region,
    width,
    location,
  }))
}
pub(super) fn new_lt(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Lt,
    region,
    width,
    location,
  }))
}
pub(super) fn new_le(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Le,
    region,
    width,
    location,
  }))
}
pub(super) fn new_gt(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Gt,
    region,
    width,
    location,
  }))
}
pub(super) fn new_ge(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, y: NodeIndex, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::BinaryOp(BinaryOp {
    operand1: x,
    operand2: y,
    ty: BinaryOpType::Ge,
    region,
    width,
    location,
  }))
}
pub(super) fn new_index(
  trans: &mut Transaction<'_, Component>, x: NodeIndex, l: NodeIndex, r: NodeIndex,
  width: usize, region: NodeIndex, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::IndexOp(IndexOp {
    operand: x,
    high: l,
    low: r,
    region,
    width,
    location,
  }))
}
pub(super) fn new_literal(
  trans: &mut Transaction<'_, Component>, x: usize, w: usize, region: NodeIndex,
  location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::Literal(Literal {
    value: usize_to_bitvec(w, x),
    width: w,
    region,
    location,
  }))
}
pub(super) fn new_true(
  trans: &mut Transaction<'_, Component>, region: NodeIndex, location: Location<'static>,
) -> NodeIndex {
  new_literal(trans, 1, 1, region, location)
}
pub(super) fn new_false(
  trans: &mut Transaction<'_, Component>, region: NodeIndex, location: Location<'static>,
) -> NodeIndex {
  new_literal(trans, 0, 1, region, location)
}
pub(super) fn new_reduce_sum(
  trans: &mut Transaction<'_, Component>, xs: &Vec<NodeIndex>, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::ReduceOp(ReduceOp {
    operands: xs.clone(),
    ty: ReduceOpType::Sum,
    width,
    region,
    location,
  }))
}
pub(super) fn new_reduce_and(
  trans: &mut Transaction<'_, Component>, xs: &Vec<NodeIndex>, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::ReduceOp(ReduceOp {
    operands: xs.clone(),
    ty: ReduceOpType::And,
    width,
    region,
    location,
  }))
}
pub(super) fn new_reduce_or(
  trans: &mut Transaction<'_, Component>, xs: &Vec<NodeIndex>, region: NodeIndex,
  width: usize, location: Location<'static>,
) -> NodeIndex {
  trans.new_node(Component::ReduceOp(ReduceOp {
    operands: xs.clone(),
    ty: ReduceOpType::Or,
    width,
    region,
    location,
  }))
}
// pub(super) fn new_reduce_cat(
//   trans: &mut Transaction<'_, Component>, xs: &Vec<NodeIndex>, region: RegionId,
//   width: usize,
// ) -> NodeIndex {
//   trans.new_node(Component::ReduceOp(ReduceOp {
//     operands: xs.clone(),
//     ty: ReduceOpType::Cat,
//     width,
//     region,
//   }))
// }
