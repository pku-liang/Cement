use std::fmt::Debug;

use visible::StructFields;

use super::events::*;

#[StructFields(pub)]
pub struct SimCycle {
  comb_events: Vec<BoxEvent>,
  reg_events: Vec<BoxEvent>,
  poke_events: Vec<BoxEvent>,
  keep_poke_events: Vec<BoxEvent>,
  peek_events: Vec<BoxEvent>,
}

impl SimCycle {
  pub fn empty() -> Self {
    SimCycle {
      comb_events: Vec::new(),
      reg_events: Vec::new(),
      poke_events: Vec::new(),
      keep_poke_events: Vec::new(),
      peek_events: Vec::new(),
    }
  }

  pub fn run(&mut self) {
    // println!("{:?}", self);
    for evt in &self.keep_poke_events {
      evt.run();
    }
    for evt in &self.poke_events {
      evt.run();
    }
    self.peek_events.clear();
    for evt in &self.comb_events {
      evt.run();
    }
    for evt in &self.reg_events {
      evt.run();
    }
    for evt in &self.peek_events {
      evt.run();
    }
    self.peek_events.clear();
  }

  pub fn merge(&mut self, other: Self) {
    self.comb_events.extend(other.comb_events.into_iter());
    self.reg_events.extend(other.reg_events.into_iter());
    self.poke_events.extend(other.poke_events.into_iter());
    self.keep_poke_events.extend(other.keep_poke_events.into_iter());
    self.peek_events.extend(other.peek_events.into_iter());
  }
}

impl Debug for SimCycle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "SimCycle {{\n  comb_events: [\n")?;
    for evt in &self.comb_events {
      write!(f, "    {:?},\n", evt)?;
    }
    write!(f, "  ],\nreg_events:[\n")?;
    for evt in &self.reg_events {
      write!(f, "    {:?},\n", evt)?;
    }
    write!(f, "  ],\npoke_events:[\n")?;
    for evt in &self.poke_events {
      write!(f, "    {:?},\n", evt)?;
    }
    write!(f, "  ],\nkeep_poke_events:[\n")?;
    for evt in &self.keep_poke_events {
      write!(f, "    {:?},\n", evt)?;
    }
    write!(f, "  ],\npeek_events:[\n")?;
    for evt in &self.peek_events {
      write!(f, "    {:?},\n", evt)?;
    }
    write!(f, "}}\n")?;
    Ok(())
  }
}
