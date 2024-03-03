//! Passes on GIR

use core::panic;
use std::collections::{hash_map, HashMap, HashSet};
use std::panic::Location;

use irony_cmt::{
  self, AttributeEnum, BoolAttr, ConstantAttr, DataTypeEnum, Entity, EntityEnum,
  EntityId, Environ, IREvent, IRWire, LocationAttr, OpEnum, OpId, RegionId, StringAttr,
  UIntType,
};
use tgraph::typed_graph::{Context, Graph, NodeIndex, Transaction};

use super::build_fsm::*;
use super::component::{Component, *};
use super::construction::*;
use crate::compiler::Cmtc;
use crate::utils::*;

struct TmpStorage {
  entity2node: HashMap<usize, NodeIndex>,
  region2node: HashMap<usize, NodeIndex>,
}
impl TmpStorage {
  fn get_entity(&self, entity: EntityId) -> NodeIndex {
    *self.entity2node.get(&entity.0).unwrap()
  }

  fn get_region(&self, region: RegionId) -> NodeIndex {
    *self.region2node.get(&region.0).unwrap()
  }
}

pub fn all_passes(cmtc: &mut Cmtc) -> Graph<Component> {
  let ctx = Context::new();
  let mut graph = Graph::<Component>::new(&ctx);
  let mut tmp = TmpStorage {
    entity2node: HashMap::new(),
    region2node: HashMap::new(),
  };

  // eprintln!("Load Regions");
  graph.commit(load_regions(cmtc, &ctx, &mut tmp));
  // eprintln!("Load Entities");
  graph.commit(load_entities(cmtc, &ctx, &mut tmp));
  // eprintln!("Get event signal");
  graph.commit(get_event_signal(cmtc, &ctx, &mut tmp));
  // eprintln!("Load Ast");
  graph.commit(load_ast(cmtc, &ctx, &graph, &tmp));
  // eprintln!("Make FSM");
  graph.commit(make_fsms(&ctx, &graph));
  // eprintln!("Generate go done");
  graph.commit(generate_go_done(&ctx, &graph));
  // eprintln!("Fsm encoding 1");
  graph.commit(fsm_encoding_1(&ctx, &graph));
  // eprintln!("Fsm encoding 2");
  graph.commit(fsm_encoding_2(&ctx, &graph));
  // eprintln!("State encode expr");
  graph.commit(state_encode_expr(&ctx, &graph));
  // eprintln!("Make state event");
  graph.commit(make_state_event(&ctx, &graph));
  // eprintln!("Make transition event");
  graph.commit(make_transition_event(&ctx, &graph));
  // eprintln!("Merge event trigger");
  graph.commit(merge_event_trigger(&ctx, &graph));

  // eprintln!("Cond0 prop");
  // while let Some(trans) = cond0prop(&ctx, &graph) {
  // graph.commit(trans);
  // }

  // eprintln!("Remove reduce");
  graph.commit(replace_reduce(&ctx, &graph));

  // eprintln!("Expr2Wire");
  graph.commit(expr2wire(&ctx, &graph));

  // !eprintln("Merge select node");
  graph.commit(merge_select_node(&ctx, &graph));

  return graph;
}

fn load_regions<'a>(
  cmtc: &Cmtc, ctx: &Context, tmp: &mut TmpStorage,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for (id, _) in cmtc.ir.region_table.iter() {
    tmp.region2node.insert(
      *id,
      trans.new_node(Component::Region(Region { region: Some(RegionId(*id)) })),
    );
  }

  trans
}

fn load_entities<'a>(
  cmtc: &Cmtc, ctx: &Context, tmp: &mut TmpStorage,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for entity in cmtc.ir.entity_table.iter() {
    match entity.1 {
      EntityEnum::IREvent(event) => {
        let x = Event {
          parent_id: tmp.get_region(event.parent.unwrap()),
          entity_id: EntityId(event.id),
          signal: NodeIndex::empty(),
          location: event.location.as_ref().unwrap().0,
        };
        tmp.entity2node.insert(event.id, trans.new_node(Component::Event(x)));
      },
      EntityEnum::IRWire(wire) => {
        let width = wire.dtype.as_ref().unwrap().width();
        let x = Wire {
          width,
          region: tmp.get_region(wire.parent.unwrap()),
          entity_id: Some(EntityId(wire.id)),
          location: wire.location.as_ref().unwrap().0,
        };
        tmp.entity2node.insert(wire.id, trans.new_node(Component::Wire(x)));
      },
      _ => {},
    }
  }

  return trans;
}

fn get_event_signal<'a>(
  cmtc: &Cmtc, ctx: &Context, tmp: &mut TmpStorage,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for (_, op) in cmtc.ir.op_table.iter() {
    if let OpEnum::EventSignal(event) = op {
      let e = tmp.get_entity(event.event.unwrap());
      let s = tmp.get_entity(event.signal.unwrap());
      trans.mut_node(e, move |x| {
        if let Component::Event(y) = x {
          y.signal = s;
        }
      })
    }
  }

  trans
}
fn load_ast<'a>(
  cmtc: &Cmtc, ctx: &Context, graph: &Graph<Component>, tmp: &TmpStorage,
) -> Transaction<'a, Component> {
  let mut trans: Transaction<'_, Component> = Transaction::new(ctx);
  let mut entity_ids = HashMap::new();
  let mut synth_ids = HashMap::new();

  for op in cmtc.ir.op_table.iter() {
    match op.1 {
      OpEnum::StmtSynth(x) => {
        let id = trans.alloc_node();
        synth_ids.insert(x.id, id);
      },
      OpEnum::StmtSeq(x) => {
        let id = trans.alloc_node();
        entity_ids.insert(x.lhs.unwrap().0, id);
        // reserved_ids.insert(x.id, trans.alloc_node());
      },
      OpEnum::StmtIf(x) => {
        entity_ids.insert(x.lhs.unwrap().0, trans.alloc_node());
      },
      OpEnum::StmtFor(x) => {
        entity_ids.insert(x.lhs.unwrap().0, trans.alloc_node());
      },
      OpEnum::StmtWhile(x) => {
        entity_ids.insert(x.lhs.unwrap().0, trans.alloc_node());
      },
      OpEnum::StmtPar(x) => {
        entity_ids.insert(x.lhs.unwrap().0, trans.alloc_node());
      },
      OpEnum::StmtStep(step) => {
        entity_ids.insert(
          step.lhs.unwrap().0,
          trans.new_node(Component::AstStep(AstStep {
            events: step
              .events
              .iter()
              .map(|entity| tmp.get_entity(entity.unwrap()))
              .collect(),
            waits: step
              .wait_at_exist
              .iter()
              .map(|entity| tmp.get_entity(entity.unwrap()))
              .collect(),
            location: cmtc_get_entity_location(cmtc, step.lhs.unwrap()),
          })),
        );
      },
      _ => {},
    }
  }
  for op in cmtc.ir.op_table.iter() {
    match op.1 {
      OpEnum::StmtSynth(synth) => {
        let node = *synth_ids.get(&synth.id).unwrap();

        trans.fill_back_node(
          node,
          Component::AstSynth(AstSynth {
            body: entity_ids[&synth.stmt.unwrap().0],
            clock: synth.clk.unwrap(),
            prot_evts: synth
              .protocol_events
              .iter()
              .map(|entity| *tmp.entity2node.get(&entity.unwrap().0).unwrap())
              .collect(),
            region: tmp.get_region(synth.parent.unwrap()),
            location: cmtc_get_entity_location(cmtc, synth.stmt.unwrap()),
          }),
        );
      },
      OpEnum::StmtSeq(seq) => {
        let node = *entity_ids.get(&seq.lhs.unwrap().0).unwrap();
        trans.fill_back_node(
          node,
          Component::AstSeq(AstSeq {
            children: seq
              .sub_stmts
              .iter()
              .map(|entity| entity_ids[&entity.unwrap().0])
              .collect(),
            location: cmtc_get_entity_location(cmtc, seq.lhs.unwrap()),
          }),
        );
      },
      OpEnum::StmtIf(stmt_if) => {
        let node = *entity_ids.get(&stmt_if.lhs.unwrap().0).unwrap();
        let cond = Event::get_by_type(graph, tmp.get_entity(stmt_if.cond.unwrap()))
          .unwrap()
          .signal;
        if let Some(alt) = stmt_if.else_stmt {
          trans.fill_back_node(
            node,
            Component::AstIfElse(AstIfElse {
              cond,
              then: entity_ids[&stmt_if.then_stmt.unwrap().0],
              alt: entity_ids[&alt.0],
              location: cmtc_get_entity_location(cmtc, stmt_if.lhs.unwrap()),
            }),
          );
        } else {
          trans.fill_back_node(
            node,
            Component::AstIf(AstIf {
              cond,
              then: entity_ids[&stmt_if.then_stmt.unwrap().0],
              location: cmtc_get_entity_location(cmtc, stmt_if.lhs.unwrap()),
            }),
          );
        }
      },
      OpEnum::StmtFor(stmt_for) => {
        let node = *entity_ids.get(&stmt_for.lhs.unwrap().0).unwrap();
        // TODO: remove this +1
        let width = Wire::get_by_type(graph, tmp.get_entity(stmt_for.indvar_wr.unwrap()))
          .unwrap()
          .width
          + 1;

        trans.fill_back_node(
          node,
          Component::AstFor(AstFor {
            loop_var_rd: tmp.get_entity(stmt_for.indvar_rd.unwrap()),
            loop_var_wr: tmp.get_entity(stmt_for.indvar_wr.unwrap()),
            // TODO: get width of loop_var
            loop_var_width: width,
            body: entity_ids[&stmt_for.do_stmt.unwrap().0],
            start: if let Some(start) = stmt_for.start {
              tmp.get_entity(start)
            } else {
              NodeIndex::empty()
            },
            end: if let Some(end) = stmt_for.end {
              tmp.get_entity(end)
            } else {
              NodeIndex::empty()
            },
            step: NodeIndex::empty(),
            c_start: if let Some(x) = &stmt_for.const_start {
              x.0.try_into().unwrap()
            } else {
              0
            },
            c_end: if let Some(x) = &stmt_for.const_end {
              x.0.try_into().unwrap()
            } else {
              0
            },
            c_step: if let Some(x) = &stmt_for.const_step {
              x.0.try_into().unwrap()
            } else {
              0
            },
            location: cmtc_get_entity_location(cmtc, stmt_for.lhs.unwrap()),
          }),
        );
      },
      OpEnum::StmtWhile(stmt_while) => {
        let node = *entity_ids.get(&stmt_while.lhs.unwrap().0).unwrap();
        let cond = Event::get_by_type(graph, tmp.get_entity(stmt_while.cond.unwrap()))
          .unwrap()
          .signal;
        trans.fill_back_node(
          node,
          Component::AstWhile(AstWhile {
            body: entity_ids[&stmt_while.do_stmt.unwrap().0],
            cond,
            location: cmtc_get_entity_location(cmtc, stmt_while.lhs.unwrap()),
          }),
        );
      },
      OpEnum::StmtPar(par) => {
        let node = *entity_ids.get(&par.lhs.unwrap().0).unwrap();
        trans.fill_back_node(
          node,
          Component::AstPar(AstPar {
            children: par
              .stmts
              .iter()
              .map(|entity| entity_ids[&entity.unwrap().0])
              .collect(),
            location: cmtc_get_entity_location(cmtc, par.lhs.unwrap()),
          }),
        );
      },
      _ => {},
    }
  }

  trans
}

fn make_fsms<'a>(ctx: &Context, graph: &Graph<Component>) -> Transaction<'a, Component> {
  let mut trans: Transaction<'_, Component> = Transaction::new(ctx);

  for (_, synth) in AstSynth::iter_by_type(graph) {
    let (idle_state, state_root, mut fsm) =
      traverse_ast(ctx, synth.body, graph, synth.region);

    let mut fsm_trans = Transaction::new(&ctx);
    let idle_state_node =
      fsm_trans.new_node(Component::LeafNode(LeafNode { state: idle_state }));
    let state_root = fsm_trans.new_node(Component::ExcNode(ExcNode {
      children: vec![idle_state_node, state_root],
      encoding: Vec::new(),
    }));
    fsm.commit(fsm_trans);

    let states = HashSet::from_iter(State::iter_by_type(&fsm).map(|(i, _)| i));
    let transitions = HashSet::from_iter(Transition::iter_by_type(&fsm).map(|(i, _)| i));
    trans.merge_graph(fsm);
    let fsm = FSM {
      ast: synth.body,
      state_root,
      idle_state,
      state_reg: NodeIndex::empty(),
      states,
      transitions,
      go: synth.prot_evts[0],
      done: synth.prot_evts[1],
      clock: synth.clock,
      region: synth.region,
      location: synth.location,
    };
    trans.new_node(Component::FSM(fsm));
  }

  trans
}

fn generate_go_done<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans: Transaction<'_, Component> = Transaction::new(ctx);

  // let mut trigger_map = HashMap::new();
  // for (x, trigger) in EventTrigger::iter_by_type(graph) {
  //   trigger_map.insert(trigger.event, x);
  // }

  for (_, fsm) in FSM::iter_by_type(graph) {
    // let idle_state = State::get_by_type(graph, fsm.idle_state).unwrap();
    let go_event = Event::get_by_type(graph, fsm.go).unwrap();
    // let done_event = Event::get_by_type(graph, fsm.done).unwrap();

    let go_wire = new_wire(&mut trans, 1, fsm.region, fsm.location);
    let go_eval = trans.new_node(Component::EventEval(EventEval {
      eval: go_wire,
      parent_id: go_event.parent_id,
      event: fsm.go,
    }));
    trans.redirect_node(fsm.go, go_eval);

    // let done_wire = new_wire(&mut trans, 1, fsm.region, fsm.location);
    // let done_trigger = trans.new_node(Component::EventTrigger(EventTrigger {
    //   trigger: done_wire,
    //   parent_id: done_event.parent_id,
    //   event: fsm.done,
    // }));

    for (transit_idx, transit) in Transition::iter_by_type(graph) {
      if hashset_only_element(&transit.froms, &fsm.idle_state) {
        let cond =
          new_and(&mut trans, transit.cond, go_wire, fsm.region, 1, fsm.location);
        trans.mut_node(transit_idx, move |x| {
          if let Component::Transition(y) = x {
            y.cond = cond;
          }
        });
      }
    }
    // TODO: maintian idle_state.tos
    // for transit_idx in &idle_state.tos {
    //   let transit = Transition::get_by_type(graph, *transit_idx).unwrap();
    //   let cond = new_and(&mut trans, transit.cond, go_wire, fsm.region, 1);
    //   trans.mut_node(*transit_idx, move |x| {
    //     if let Component::Transition(y) = x {
    //       y.cond = cond;
    //     }
    //   });
    // }
  }

  trans
}
fn fsm_encoding_1<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);
  for (i, fsm) in FSM::iter_by_type(graph) {
    let state_bits = traverse_encoding_1(fsm.state_root, graph, &mut trans);
    let state_reg = new_state_reg(&mut trans, state_bits, fsm.region, fsm.location);
    trans.mut_node(i, move |x| {
      let Component::FSM(y) = x else {
        panic!("Not possible!");
      };
      y.state_reg = state_reg;
    });
  }
  trans
}
fn fsm_encoding_2<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);
  for (_, fsm) in FSM::iter_by_type(graph) {
    traverse_encoding_2(fsm.state_root, graph, &mut trans, 0, Vec::new());
  }
  trans
}

fn traverse_encoding_1<'a>(
  root: NodeIndex, graph: &Graph<Component>, trans: &mut Transaction<'a, Component>,
) -> usize {
  let node = graph.get_node(root).unwrap();
  match node {
    Component::LeafNode(_) => 0,
    Component::ExcNode(exc) => {
      let mut bits = Vec::new();
      for c in exc.children.iter() {
        bits.push(traverse_encoding_1(*c, graph, trans));
      }
      let max_bits = bits.iter().max().unwrap();

      let cur_bits = clog2(exc.children.len());
      trans.mut_node(root, move |node| {
        let Component::ExcNode(x) = node else {
          panic!("Not possible");
        };
        for i in 0..x.children.len() {
          x.encoding.push((0, cur_bits, i));
        }
      });
      max_bits + cur_bits
    },
    Component::ParNode(par) => {
      let mut bits = Vec::new();
      for c in par.children.iter() {
        bits.push(traverse_encoding_1(*c, graph, trans));
      }
      let sum_bits: usize = bits.iter().sum();

      trans.mut_node(root, move |node| {
        let Component::ParNode(x) = node else {
          panic!("Not possible");
        };
        let mut sum = 0;
        for b in bits {
          x.encoding.push((sum, sum + b));
          sum = sum + b;
        }
      });

      sum_bits
    },
    _ => panic!("Not a valid node in state tree!"),
  }
}
fn traverse_encoding_2<'a>(
  root: NodeIndex, graph: &Graph<Component>, trans: &mut Transaction<'a, Component>,
  offset: usize, encoding: Vec<(usize, usize, usize)>,
) {
  let node = graph.get_node(root).unwrap();
  // eprintln!("Encoding2 {:?}", node);
  match node {
    Component::LeafNode(leaf) => {
      let state = State::get_by_type(graph, leaf.state).unwrap();
      let new_state = trans.new_node(Component::EncodedState(EncodedState {
        events: state.events.clone(),
        encoding,
        match_wire: NodeIndex::empty(),
      }));
      trans.redirect_node(leaf.state, new_state);
      // eprintln!("Replaced!");
    },
    Component::ExcNode(exc) => {
      for (c, b) in exc.children.iter().zip(exc.encoding.iter()) {
        let mut e = encoding.clone();
        e.push((b.0 + offset, b.1 + offset, b.2));
        traverse_encoding_2(*c, graph, trans, offset + b.1, e);
      }
    },
    Component::ParNode(par) => {
      for (c, b) in par.children.iter().zip(par.encoding.iter()) {
        let e = encoding.clone();
        traverse_encoding_2(*c, graph, trans, offset + b.0, e);
      }
    },
    _ => {},
  }
}

// fn replace_empty_cond<'a>(
//   ctx: &Context, graph: &Graph<Component>,
// ) -> Transaction<'a, Component> {
//   let mut trans = Transaction::new(ctx);
//   let lit_true = new_literal(&mut trans, 1, 1, RegionId(0));

//   for (i, transit) in Transition::iter_by_type(graph) {
//     if transit.cond.is_empty() {
//       trans.mut_node(i, move |x| {
//         if let Component::Transition(y) = x {
//           y.cond = lit_true;
//         }
//       })
//     }
//   }

//   trans
// }

fn state_encode_expr<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);
  for (_, fsm) in FSM::iter_by_type(graph) {
    let state_reg = StateReg::get_by_type(graph, fsm.state_reg).unwrap();
    for state_idx in &fsm.states {
      // eprintln!("{:?}", graph.get_node(*state_idx));
      let state = EncodedState::get_by_type(graph, *state_idx).unwrap();
      let mut exprs = Vec::new();
      for (start, end, encode) in &state.encoding {
        let bits = usize_to_bitvec(end - start, *encode);
        for (i, b) in bits.iter().enumerate() {
          if *b {
            exprs.push(state_reg.wire_out[start + i]);
          } else {
            exprs.push(new_not(
              &mut trans,
              state_reg.wire_out[start + i],
              fsm.region,
              1,
              fsm.location,
            ));
          }
        }
      }
      let match_wire = new_reduce_and(&mut trans, &exprs, fsm.region, 1, fsm.location);
      trans.mut_node(*state_idx, move |x| {
        if let Component::EncodedState(y) = x {
          y.match_wire = match_wire;
        }
      });
    }
  }
  trans
}

fn make_state_event<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for (_, fsm) in FSM::iter_by_type(graph) {
    let idle_state = EncodedState::get_by_type(graph, fsm.idle_state).unwrap();
    let done_event = Event::get_by_type(graph, fsm.done).unwrap();
    new_assign(&mut trans, done_event.signal, idle_state.match_wire, fsm.region);

    for state_idx in &fsm.states {
      let state = EncodedState::get_by_type(graph, *state_idx).unwrap();
      for event_idx in &state.events {
        let event = Event::get_by_type(graph, *event_idx).unwrap();
        let trigger = trans.new_node(Component::EventTrigger(EventTrigger {
          trigger: state.match_wire,
          parent_id: event.parent_id,
          event: *event_idx,
        }));
        trans.redirect_node(*event_idx, trigger);
      }
    }
  }

  trans
}

fn make_transition_event<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);
  let mut to_remove = HashSet::new();

  for (_, fsm) in FSM::iter_by_type(graph) {
    let state_reg = StateReg::get_by_type(graph, fsm.state_reg).unwrap();
    let lit_one = new_literal(&mut trans, 1, 1, fsm.region, fsm.location);
    let lit_zero = new_literal(&mut trans, 0, 1, fsm.region, fsm.location);

    //Default assign
    for (wi, wo) in state_reg.wire_in.iter().zip(state_reg.wire_out.iter()) {
      new_assign(&mut trans, *wi, *wo, fsm.region);
    }

    for transit_idx in &fsm.transitions {
      let transit = Transition::get_by_type(graph, *transit_idx).unwrap();

      let mut cond_wires = Vec::from_iter(
        transit
          .froms
          .iter()
          .map(|x| EncodedState::get_by_type(graph, *x).unwrap().match_wire),
      );
      if !transit.cond.is_empty() {
        cond_wires.push(transit.cond);
      }

      let event_cond =
        new_reduce_and(&mut trans, &cond_wires, fsm.region, 1, fsm.location);
      let region = trans.new_node(Component::Region(Region { region: None }));
      let mut event = GenEvent {
        cond: event_cond,
        assigns: HashSet::new(),
        entity_id: None,
        region: fsm.region,
        child_region: region,
        location: fsm.location,
      };
      let event_idx = trans.alloc_node();

      for x in &transit.acts {
        let assign = Assign::get_by_type(graph, *x).unwrap();
        event.assigns.insert(trans.new_node(Component::CondAssign(CondAssign {
          lhs: assign.lhs,
          rhs: assign.rhs,
          cond: event_idx,
          region,
          location: fsm.location,
        })));
        to_remove.insert(*x);
      }

      let to_state = EncodedState::get_by_type(graph, transit.to).unwrap();
      for (start, end, encode) in &to_state.encoding {
        let bits = usize_to_bitvec(end - start, *encode);
        for (i, b) in bits.iter().enumerate() {
          event.assigns.insert(trans.new_node(Component::CondAssign(CondAssign {
            lhs: state_reg.wire_in[start + i],
            rhs: if *b { lit_one } else { lit_zero },
            cond: event_idx,
            region,
            location: fsm.location,
          })));
        }
      }

      trans.fill_back_node(event_idx, Component::GenEvent(event));
    }
  }
  for x in to_remove {
    trans.remove_node(x);
  }

  trans
}

fn cond0prop<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Option<Transaction<'a, Component>> {
  let mut trans = Transaction::new(ctx);
  let mut changed = false;

  for (i, node) in BinaryOp::iter_by_type(graph) {
    if node.operand1.is_empty() && node.operand2.is_empty() {
      trans.redirect_node(i, NodeIndex::empty());
      changed = true;
    }
  }
  for (i, node) in ReduceOp::iter_by_type(graph) {
    if node.operands.iter().find(|x| !x.is_empty()).is_none() {
      trans.redirect_node(i, NodeIndex::empty());
      changed = true;
    }
  }
  if changed {
    Some(trans)
  } else {
    None
  }
}
fn replace_reduce<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for (id, reduce) in ReduceOp::iter_by_type(graph) {
    let root = make_tree_reduce(
      &reduce.operands,
      &mut trans,
      reduce2binary_op(reduce.ty),
      reduce.region,
      1,
      reduce.location,
    );
    trans.redirect_all_node(id, root);
    trans.remove_node(id);
  }

  trans
}

fn expr2wire<'a>(ctx: &Context, graph: &Graph<Component>) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  for (i, node) in graph.iter_nodes() {
    match node {
      Component::Literal(x) => {
        let result = new_wire(&mut trans, x.width, x.region, x.location);
        new_assign(&mut trans, result, *i, x.region);
        trans.redirect_node(*i, result);
      },
      Component::UnaryOp(x) => {
        let result = new_wire(&mut trans, x.width, x.region, x.location);
        new_assign(&mut trans, result, *i, x.region);
        trans.redirect_node(*i, result);
      },
      Component::BinaryOp(x) => {
        let result = new_wire(&mut trans, x.width, x.region, x.location);
        new_assign(&mut trans, result, *i, x.region);
        trans.redirect_node(*i, result);
      },
      Component::IndexOp(x) => {
        let result = new_wire(&mut trans, x.width, x.region, x.location);
        new_assign(&mut trans, result, *i, x.region);
        trans.redirect_node(*i, result);
      },
      _ => {},
    }
  }
  trans
}

fn merge_event_trigger<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trigger_map = HashMap::new();
  let mut trigger_node_map = HashMap::new();
  let mut event_region = HashMap::new();
  let mut trans = Transaction::new(ctx);
  for (id, x) in EventTrigger::iter_by_type(graph) {
    trigger_map.entry(x.event).or_insert(Vec::new()).push(x.trigger);
    trigger_node_map.entry(x.event).or_insert(Vec::new()).push(id);
    event_region.insert(x.event, x.parent_id);
  }
  for (evt, triggers) in trigger_map {
    if triggers.len() > 1 {
      let location = Event::get_by_type(graph, evt).unwrap().location;
      let reduced = new_reduce_or(&mut trans, &triggers, event_region[&evt], 1, location);
      for node in &trigger_node_map[&evt] {
        trans.remove_node(*node);
      }
      trans.new_node(Component::EventTrigger(EventTrigger {
        parent_id: event_region[&evt],
        trigger: reduced,
        event: evt,
      }));
    }
  }
  trans
}

fn merge_select_node<'a>(
  ctx: &Context, graph: &Graph<Component>,
) -> Transaction<'a, Component> {
  let mut trans = Transaction::new(ctx);

  let mut default_assign = HashMap::new();
  for (id, assign) in Assign::iter_by_type(graph) {
    if let hash_map::Entry::Vacant(x) = default_assign.entry(assign.lhs) {
      x.insert((id, assign.rhs));
    } else {
      panic!("Multiple assign into the same lhs!");
    }
  }

  let mut cond_assigns = HashMap::new();
  for (_, cassign) in CondAssign::iter_by_type(graph) {
    let lhs = Wire::get_by_type(graph, cassign.lhs).unwrap();
    let wire = new_wire(&mut trans, lhs.width, cassign.region, cassign.location);
    new_assign(&mut trans, wire, cassign.rhs, cassign.region);

    cond_assigns.entry(cassign.lhs).or_insert(Vec::new()).push((cassign.cond, wire));
  }

  for (lhs, cases) in cond_assigns {
    let (assign_id, default_val) =
      *default_assign.get(&lhs).expect("No default value for select!");
    trans.remove_node(assign_id);
    let region = Wire::get_by_type(graph, lhs).unwrap().region;
    trans.new_node(Component::Select(Select {
      parent_id: region,
      lhs,
      events: cases.iter().map(|x| x.0).collect(),
      rhs: cases.iter().map(|x| x.1).collect(),
      default_val,
    }));
  }

  trans
}

fn make_tree_reduce<'a>(
  nodes: &[NodeIndex], trans: &mut Transaction<'a, Component>, ty: BinaryOpType,
  region: NodeIndex, width: usize, location: Location<'static>,
) -> NodeIndex {
  if nodes.len() == 1 {
    nodes[0]
  } else if nodes.len() == 2 {
    // let result = new_wire(trans, width, region, location);
    let binary = trans.new_node(Component::BinaryOp(BinaryOp {
      operand1: nodes[0],
      operand2: nodes[1],
      ty,
      region,
      width,
      location,
    }));
    // new_assign(trans, result, binary, region);
    binary
  } else {
    let (l, r) = nodes.split_at(nodes.len() / 2);
    let op1 = make_tree_reduce(l, trans, ty, region, width, location);
    let op2 = make_tree_reduce(r, trans, ty, region, width, location);
    make_tree_reduce(&[op1, op2], trans, ty, region, width, location)
  }
}

pub fn retrieve_cmtc(cmtc: &mut Cmtc, graph: Graph<Component>) {
  // eprintln!("Run Retrieve");
  let mut wire_map = HashMap::new();
  let mut region_map = HashMap::new();
  let mut event_map = HashMap::new();

  // eprintln!("Retrieve Region");
  for (idx, region) in Region::iter_by_type(&graph) {
    if let Some(x) = region.region {
      region_map.insert(idx, x);
    } else {
      region_map.insert(idx, cmtc.ir.add_region(irony_cmt::Region::new(false)));
    }
  }

  // eprintln!("Retrieve Event");
  for (idx, evt) in Event::iter_by_type(&graph) {
    event_map.insert(idx, evt.entity_id);
  }

  // eprintln!("Retrieve Wire");
  for (idx, wire) in Wire::iter_by_type(&graph) {
    if wire.entity_id.is_none() {
      // eprintln!("new wire");
      let typ = DataTypeEnum::UInt(UIntType(wire.width));
      wire_map.insert(
        idx,
        cmtc_add_entity(
          cmtc,
          region_map[&wire.region],
          EntityEnum::IRWire(IRWire::new(
            Some(typ),
            Some(StringAttr(format!("GenWire{}", idx.0))),
            Some(BoolAttr(false)),
            Some(LocationAttr(wire.location)),
          )),
        ),
      );
    } else {
      wire_map.insert(idx, wire.entity_id.unwrap());
    }
  }

  // eprintln!("Retrieve State Reg");
  for (_, fsm) in FSM::iter_by_type(&graph) {
    let state_reg = StateReg::get_by_type(&graph, fsm.state_reg).unwrap();
    for (wi, wo) in state_reg.wire_in.iter().zip(state_reg.wire_out.iter()) {
      cmtc_add_op(
        cmtc,
        region_map[&fsm.region],
        OpEnum::SeqCompReg(irony_cmt::SeqCompReg::new(
          Some(wire_map[wo]),
          Some(wire_map[wi]),
          Some(fsm.clock),
          None,
          None,
        )),
      );
    }
  }

  // eprintln!("Retrieve GenEvent");
  for (idx, event) in GenEvent::iter_by_type(&graph) {
    let event_entity = cmtc_add_entity(
      cmtc,
      region_map[&event.region],
      EntityEnum::IREvent(IREvent::new(
        None,
        Some(irony_cmt::StringAttr(format!("GenEvent{}", idx.0))),
        Some(irony_cmt::BoolAttr(false)),
        Some(irony_cmt::LocationAttr(event.location)),
      )),
    );
    event_map.insert(idx, event_entity);

    cmtc_add_op(
      cmtc,
      region_map[&event.region],
      OpEnum::EventDef(irony_cmt::EventDef::new(Some(event_entity))),
    );

    cmtc_add_op(
      cmtc,
      region_map[&event.region],
      OpEnum::TmpWhen(irony_cmt::TmpWhen::new(
        Some(event_entity),
        Some(region_map[&event.child_region]),
      )),
    );

    cmtc_add_op(
      cmtc,
      region_map[&event.region],
      OpEnum::EventSignal(irony_cmt::EventSignal::new(
        Some(event_entity),
        Some(wire_map[&event.cond]),
      )),
    );
  }

  // eprintln!("Retrieve Select");
  for (_, select) in Select::iter_by_type(&graph) {
    cmtc_add_op(
      cmtc,
      region_map[&select.parent_id],
      OpEnum::TmpSelect(irony_cmt::TmpSelect::new(
        Some(wire_map[&select.lhs]),
        Some(wire_map[&select.default_val]),
        select.events.iter().map(|x| Some(event_map[x])).collect(),
        select.rhs.iter().map(|x| Some(wire_map[x])).collect(),
        Some(BoolAttr(false)),
      )),
    );
  }

  // eprintln!("Retrieve Assign");
  for (_, assign) in Assign::iter_by_type(&graph) {
    make_expr(cmtc, &graph, assign, &wire_map, region_map[&assign.region]);
  }

  // eprintln!("Retrieve EventTrigger");
  for (_, event) in EventTrigger::iter_by_type(&graph) {
    cmtc_add_op(
      cmtc,
      region_map[&event.parent_id],
      OpEnum::EventSignal(irony_cmt::EventSignal::new(
        Some(event_map[&event.event]),
        Some(wire_map[&event.trigger]),
      )),
    );
  }

  // eprintln!("Retrieve EventEval");
  for (_, event) in EventEval::iter_by_type(&graph) {
    let region_id = cmtc.ir.add_region(irony_cmt::Region::new(false));
    cmtc_add_op(
      cmtc,
      region_map[&event.parent_id],
      OpEnum::TmpWhen(irony_cmt::TmpWhen::new(
        Some(event_map[&event.event]),
        Some(region_id),
      )),
    );
    cmtc_add_op(
      cmtc,
      region_id,
      OpEnum::HwConstant(irony_cmt::HwConstant::new(
        Some(wire_map[&event.eval]),
        Some(ConstantAttr(usize_to_bitvec(1, 1))),
      )),
    );
  }
  // eprintln!("Finished!");
}

fn make_expr(
  cmtc: &mut Cmtc, graph: &Graph<Component>, assign: &Assign,
  wire_map: &HashMap<NodeIndex, EntityId>, region: RegionId,
) -> OpId {
  let lhs = wire_map[&assign.lhs];
  let rhs = graph.get_node(assign.rhs).unwrap();
  match rhs {
    Component::Literal(lit) => cmtc_add_op(
      cmtc,
      region,
      OpEnum::HwConstant(irony_cmt::HwConstant::new(
        Some(lhs),
        Some(ConstantAttr(lit.value.clone())),
      )),
    ),
    Component::BinaryOp(binary) => {
      if let Some(variadic) = binary2variadic_op(binary.ty) {
        cmtc_add_op(
          cmtc,
          region,
          OpEnum::CombVariadic(irony_cmt::CombVariadic::new(
            Some(lhs),
            vec![Some(wire_map[&binary.operand1]), Some(wire_map[&binary.operand2])],
            Some(variadic),
          )),
        )
      } else if let Some(icmp) = binary2icmp_op(binary.ty) {
        cmtc_add_op(
          cmtc,
          region,
          OpEnum::CombICmp(irony_cmt::CombICmp::new(
            Some(lhs),
            Some(wire_map[&binary.operand1]),
            Some(wire_map[&binary.operand2]),
            Some(icmp),
          )),
        )
      } else {
        panic!("Not mappable binary op!");
      }
    },
    Component::UnaryOp(unary) => cmtc_add_op(
      cmtc,
      region,
      OpEnum::TmpUnary(irony_cmt::TmpUnary::new(
        Some(lhs),
        Some(wire_map[&unary.operand]),
        Some(unary2cmtc_op(unary.ty)),
      )),
    ),
    Component::Wire(_) => cmtc_add_op(
      cmtc,
      region,
      OpEnum::Assign(irony_cmt::Assign::new(Some(lhs), Some(wire_map[&assign.rhs]))),
    ),
    _ => todo!(),
  }
}

fn cmtc_add_entity(cmtc: &mut Cmtc, region: RegionId, entity: EntityEnum) -> EntityId {
  let id = cmtc.ir.add_entity(entity);
  cmtc.ir.get_region_entry(region).and_modify(|r| r.add_entity_child(id));
  id
}
fn cmtc_add_op(cmtc: &mut Cmtc, region: RegionId, op: OpEnum) -> OpId {
  let id = cmtc.ir.add_op(op);
  cmtc.ir.get_region_entry(region).and_modify(|r| r.add_op_child(id));
  id
}

fn cmtc_get_entity_location(cmtc: &Cmtc, id: EntityId) -> Location<'static> {
  if let AttributeEnum::LocationAttr(x) =
    cmtc.ir.get_entity(id).get_attr("location").unwrap()
  {
    x.0
  } else {
    panic!()
  }
}
