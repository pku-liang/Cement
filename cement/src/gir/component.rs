//! Componts of GIR
use core::panic::Location;
use std::collections::HashSet;

use irony_cmt::{EntityId, Hash, RegionId};
use tgraph::typed_graph::*;
use tgraph_macros::*;
use visible::StructFields;

pub use super::expr::*;

#[derive(NodeEnum, Debug)]
pub enum Component {
  //   // Top-level
  //   Design(Design),
  //   Module(Module),
  // Net wire
  Region(Region),
  StateReg(StateReg),
  Wire(Wire),
  Assign(Assign),
  CondAssign(CondAssign),
  Event(Event),
  GenEvent(GenEvent),
  EventTrigger(EventTrigger),
  EventEval(EventEval),
  Select(Select),
  //   ResetEvent(ResetEvent),
  Literal(Literal),
  UnaryOp(UnaryOp),
  BinaryOp(BinaryOp),
  IndexOp(IndexOp),
  ReduceOp(ReduceOp),
  // AST
  AstStep(AstStep),
  AstSeq(AstSeq),
  AstPar(AstPar),
  AstIf(AstIf),
  AstIfElse(AstIfElse),
  // AstLoop(AstLoop),
  AstWhile(AstWhile),
  AstFor(AstFor),
  AstSynth(AstSynth),
  // FSM
  FSM(FSM),
  State(State),
  EncodedState(EncodedState),
  Transition(Transition),
  ExcNode(ExcNode),
  ParNode(ParNode),
  LeafNode(LeafNode),
}

// // Top-level
// #[derive(TypedNode, Clone)]
// #[StructFields(pub)]
// pub struct Design {
//   modules: HashSet<NodeIndex>,
// }

// #[derive(TypedNode, Clone)]
// #[StructFields(pub)]
// pub struct Module {
//   wires: HashSet<NodeIndex>,
//   ios: HashSet<NodeIndex>,
//   events: HashSet<NodeIndex>,
// }

// Net wire

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Region {
  region: Option<RegionId>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct StateReg {
  wire_in: Vec<NodeIndex>,
  wire_out: Vec<NodeIndex>,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Wire {
  width: usize,
  region: NodeIndex,
  entity_id: Option<EntityId>,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Assign {
  lhs: NodeIndex,
  rhs: NodeIndex,
  region: NodeIndex,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct CondAssign {
  lhs: NodeIndex,
  rhs: NodeIndex,
  cond: NodeIndex,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Event {
  parent_id: NodeIndex,
  entity_id: EntityId,
  signal: NodeIndex,
  location: Location<'static>
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct GenEvent {
  cond: NodeIndex,
  assigns: HashSet<NodeIndex>,
  entity_id: Option<EntityId>,
  region: NodeIndex,
  child_region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct EventTrigger {
  trigger: NodeIndex,
  parent_id: NodeIndex,
  event: NodeIndex,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct EventEval {
  eval: NodeIndex,
  parent_id: NodeIndex,
  event: NodeIndex,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Select {
  parent_id: NodeIndex,
  lhs: NodeIndex,
  events: Vec<NodeIndex>,
  rhs: Vec<NodeIndex>,
  default_val: NodeIndex,
}

// #[derive(TypedNode, Clone)]
// #[StructFields(pub)]
// pub struct ResetEvent {
//   assigns: HashSet<NodeIndex>,
//   entity_id: Option<EntityId>,
// }

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Literal {
  value: Vec<bool>,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

impl Literal{
  fn is_true(&self) -> bool {
    self.width == 1 && self.value[0]
  }

  fn is_false(&self) -> bool {
    self.width == 1 && !self.value[0]
  }
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct UnaryOp {
  operand: NodeIndex,
  ty: UnaryOpType,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct BinaryOp {
  operand1: NodeIndex,
  operand2: NodeIndex,
  ty: BinaryOpType,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct IndexOp {
  operand: NodeIndex,
  high: NodeIndex,
  low: NodeIndex,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct ReduceOp {
  operands: Vec<NodeIndex>,
  ty: ReduceOpType,
  width: usize,
  region: NodeIndex,
  location: Location<'static>,
}

// AST

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstStep {
  events: HashSet<NodeIndex>,
  waits: Vec<NodeIndex>,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstSeq {
  children: Vec<NodeIndex>,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstPar {
  children: Vec<NodeIndex>,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstIf {
  cond: NodeIndex,
  then: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstIfElse {
  cond: NodeIndex,
  then: NodeIndex,
  alt: NodeIndex,
  location: Location<'static>,
}

// #[derive(TypedNode, Clone, Debug)]
// #[StructFields(pub)]
// pub struct AstLoop {
//     body: NodeIndex,
// }

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstWhile {
  cond: NodeIndex,
  body: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstFor {
  // loop_var: NodeIndex,
  loop_var_rd: NodeIndex,
  loop_var_wr: NodeIndex,
  loop_var_width: usize,
  start: NodeIndex,
  end: NodeIndex,
  step: NodeIndex,
  c_start: usize,
  c_end: usize,
  c_step: usize,
  body: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct AstSynth {
  body: NodeIndex,
  clock: EntityId,
  prot_evts: Vec<NodeIndex>,
  region: NodeIndex,
  location: Location<'static>,
}

// FSM
#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct FSM {
  ast: NodeIndex,
  state_root: NodeIndex,
  idle_state: NodeIndex,
  state_reg: NodeIndex,
  states: HashSet<NodeIndex>,
  transitions: HashSet<NodeIndex>,
  go: NodeIndex,
  done: NodeIndex,
  clock: EntityId,
  region: NodeIndex,
  location: Location<'static>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct State {
  events: HashSet<NodeIndex>,
  //   froms: HashSet<NodeIndex>,
  //   tos: HashSet<NodeIndex>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct Transition {
  acts: HashSet<NodeIndex>,
  cond: NodeIndex,
  froms: HashSet<NodeIndex>,
  to: NodeIndex,
  event: NodeIndex,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct ExcNode {
  children: Vec<NodeIndex>,
  encoding: Vec<(usize, usize, usize)>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct ParNode {
  children: Vec<NodeIndex>,
  encoding: Vec<(usize, usize)>,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct LeafNode {
  state: NodeIndex,
}

#[derive(TypedNode, Clone, Debug)]
#[StructFields(pub)]
pub struct EncodedState {
  events: HashSet<NodeIndex>,
  encoding: Vec<(usize, usize, usize)>,
  match_wire: NodeIndex,
}
