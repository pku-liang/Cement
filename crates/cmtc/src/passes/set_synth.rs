use indexmap::IndexSet;

use super::*;

/// SetSynthPass:
/// Set the modules that are not instances of any other non-tb modules as
/// synthesis target
pub struct SetSynthPass {
  modules_as_instances: IndexSet<String>,
}

impl SetSynthPass {
  pub fn new() -> Self {
    Self {
      modules_as_instances: IndexSet::new(),
    }
  }
}

impl Visitor for SetSynthPass {
  fn name() -> &'static str {
    "set-synth"
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    // iterate over all the instances of the module
    if !data.module.is_tb() {
      for instance in data.module.instances() {
        self.modules_as_instances.insert(instance.module.clone());
      }
    }

    Ok(())
  }

  fn after_visit_modules(
    &mut self,
    circuit: &mut crate::Circuit,
  ) -> Result<(), anyhow::Error> {
    // iterate over all the modules in the circuit
    for module in circuit.modules_mut() {
      if !self.modules_as_instances.contains(&module.name)
        && !module.is_tb()
        && !module.is_virtual()
      {
        module.set_synthesis();
      }
    }
    Ok(())
  }
}
