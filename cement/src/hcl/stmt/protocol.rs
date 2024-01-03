use crate::flip;
use crate::preclude::*;

pub trait StmtProtocol {
  fn clk(&self) -> Option<EntityId>;
  fn v_name(&self) -> Vec<String>;
  fn v_event(&self) -> Vec<Event>;
}

pub struct GoDone {
  pub clk: EntityId,
  pub go: Event,
  pub done: Event,
}

#[interface(Default, Copy)]
pub struct GoDoIfc {
  pub go: B<1>,
  pub done: flip!(B<1>),
}

impl GoDone {
  pub fn new(clk: I<Clk>, go: Event, done: Event) -> Self {
    assert!(clk.v_ir_entity_id.len() == 1, "Clk wire has one entity-id");

    Self { clk: clk.v_ir_entity_id[0].unwrap(), go, done }
  }
}

impl StmtProtocol for GoDone {
  fn clk(&self) -> Option<EntityId> { Some(self.clk.to_owned()) }

  fn v_name(&self) -> Vec<String> { vec!["go".to_string(), "done".to_string()] }

  fn v_event(&self) -> Vec<Event> { vec![self.go.clone(), self.done.clone()] }
}

pub struct Pipeline {
  pub clk: EntityId,
  pub ii: usize,
}

impl Pipeline {
  pub fn new(clk: I<Clk>, ii: usize) -> Self {
    assert!(clk.v_ir_entity_id.len() == 1, "Clk wire has one entity-id");

    Self { clk: clk.v_ir_entity_id[0].unwrap(), ii }
  }
}

impl StmtProtocol for Pipeline {
  fn clk(&self) -> Option<EntityId> { Some(self.clk.to_owned()) }

  fn v_event(&self) -> Vec<Event> { Vec::new() }

  fn v_name(&self) -> Vec<String> { Vec::new() }
}
