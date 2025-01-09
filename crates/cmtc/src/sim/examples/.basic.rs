use num_bigint::BigInt;
use num_traits::Zero;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::process;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModuleInfo {
  pub full_path: String,
  pub module_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RuleInfo {
  pub module_info: ModuleInfo,
  pub rule_name: String,
  pub is_always: bool,
}

impl RuleInfo {
  pub fn new_method_rule(
    full_path: String,
    module_name: String,
    rule_name: String,
  ) -> Self {
    let module_info = ModuleInfo {
      full_path,
      module_name,
    };
    RuleInfo {
      module_info: module_info.clone(),
      rule_name,
      is_always: false,
    }
  }

  pub fn new_always_rule(
    full_path: String,
    module_name: String,
    rule_name: String,
  ) -> Self {
    let module_info = ModuleInfo {
      full_path,
      module_name,
    };
    RuleInfo {
      module_info: module_info.clone(),
      rule_name,
      is_always: true,
    }
  }
}

struct SimServer {
  rules: Vec<RuleInfo>,
  functions: Vec<fn(&mut SimServer, Vec<BigInt>) -> Vec<BigInt>>,
  module_map: HashMap<ModuleInfo, Vec<usize>>,
  schedule_map: HashMap<ModuleInfo, VecDeque<usize>>,
  rule_run: Vec<bool>,
  guards: Vec<Vec<Guard>>,
  method_top: HashMap<usize, Vec<usize>>, /* for method rule, record the
                                           * top-level always rule */
  always_rule_alive: HashMap<usize, bool>, /* for always rule, record
                                            * whether it is abandoned
                                            * because the guard is not
                                            * satisfied */
  reg_value_map: HashMap<String, BigInt>,
  wire_value_map: HashMap<String, BigInt>,
  integer_value_map: HashMap<String, BigInt>,
}

#[derive(Clone, Debug)]
enum Guard {
  Explicit(fn(&mut SimServer) -> bool),
  Implicit(RunStatus),
}

#[derive(Clone, Debug)]
enum RunStatus {
  Run(usize),
  NotRun(usize),
}

impl SimServer {
  pub fn new() -> Self {
    SimServer {
      rules: Vec::new(),
      functions: Vec::new(),
      module_map: HashMap::new(),
      schedule_map: HashMap::new(),
      rule_run: Vec::new(),
      guards: Vec::new(),
      method_top: HashMap::new(),
      always_rule_alive: HashMap::new(),
      reg_value_map: HashMap::new(),
      wire_value_map: HashMap::new(),
      integer_value_map: HashMap::new(),
    }
  }

  pub fn reset_server_status(&mut self) {
    self.schedule_map.clear();
    for value in self.rule_run.iter_mut() {
      *value = false;
    }
    for value in self.always_rule_alive.values_mut() {
      *value = false;
    }
  }

  pub fn find_rule_id(&self, rule_info: &RuleInfo) -> usize {
    self
      .rules
      .iter()
      .position(|rule| rule == rule_info)
      .unwrap()
  }

  pub fn add_rule(
    &mut self,
    full_path: String,
    module_name: String,
    rule_name: String,
    is_always: bool,
    is_private: bool,
  ) -> usize {
    let module_info = ModuleInfo {
      full_path,
      module_name,
    };
    let rule_info = RuleInfo {
      module_info: module_info.clone(),
      rule_name,
      is_always,
    };
    self.rules.push(rule_info);
    self.rule_run.push(false);
    let rule_id = self.rules.len() - 1;
    if is_always {
      self.always_rule_alive.insert(rule_id, false);
    }

    // Update the HashMap with the rule ID
    if !is_private {
      self
        .module_map
        .entry(module_info)
        .or_insert_with(Vec::new)
        .push(rule_id);
    }

    rule_id
  }

  pub fn add_function(
    &mut self,
    func: fn(&mut SimServer, Vec<BigInt>) -> Vec<BigInt>,
  ) {
    self.functions.push(func);
  }

  pub fn add_guard_vec(&mut self, guard_vec: Vec<Guard>) {
    self.guards.push(guard_vec); // Add the Vec<Guard> to the guards Vec
  }

  pub fn add_method_top_rule(
    &mut self,
    method_rule_id: usize,
    top_rule_id: usize,
  ) {
    if let Some(top_rules) = self.method_top.get_mut(&method_rule_id) {
      top_rules.push(top_rule_id);
    } else {
      self.method_top.insert(method_rule_id, vec![top_rule_id]);
    }
  }

  pub fn is_guard_satisfied(&mut self, rule_id: usize) -> bool {
    if let Some(guard_vec) = self.guards.clone().get(rule_id) {
      for guard in guard_vec {
        match guard {
          Guard::Explicit(func) => {
            if !func(self) {
              return false; // Guard failed, return false immediately
            }
          }
          Guard::Implicit(RunStatus::Run(sub_rule_id)) => {
            // Recursively check the guard for the sub-rule
            if !self.is_guard_satisfied(*sub_rule_id) {
              return false;
            }
          }
          Guard::Implicit(RunStatus::NotRun(sub_rule_id)) => {
            if let Some(&true) = self.rule_run.get(*sub_rule_id) {
              return false;
            }
          }
        }
      }
    }

    // If all guards passed, return true
    true
  }

  pub fn schedule_method(
    &mut self,
    rule_info: RuleInfo,
    args: Vec<BigInt>,
  ) -> Vec<BigInt> {
    let module_info = rule_info.module_info.clone();
    if !self.schedule_map.contains_key(&module_info) {
      if let Some(rules_vec) = self.module_map.get(&module_info) {
        let rules_queue: VecDeque<usize> = rules_vec.iter().cloned().collect();
        self.schedule_map.insert(module_info, rules_queue);
      }
    }
    let rule_id = self.find_rule_id(&rule_info);
    if let Some(rule_run) = self.rule_run.get_mut(rule_id) {
      *rule_run = true;
    }
    if let Some(function) = self.functions.get(rule_id) {
      return function(self, args);
    }
    vec![]
  }

  pub fn add_reg(&mut self, key: String) {
    self.reg_value_map.insert(key, BigInt::zero());
  }

  pub fn add_wire(&mut self, key: String) {
    self.wire_value_map.insert(key, BigInt::zero());
  }

  pub fn add_integer(&mut self, key: String) {
    self.integer_value_map.insert(key, BigInt::zero());
  }
}

struct SimWhole {
  sim_server: SimServer,
}

impl SimWhole {
  pub fn new() -> Self {
    SimWhole {
      sim_server: SimServer::new(),
    }
  }

  pub fn start(&mut self, top_module: ModuleInfo) {
    // copy top module to the schedule queue
    if let Some(rules_vec) = self.sim_server.module_map.get(&top_module) {
      let rules_queue: VecDeque<usize> = rules_vec.iter().cloned().collect();
      self.sim_server.schedule_map.insert(top_module, rules_queue);
    }
  }

  pub fn schedule(&mut self) {
    let mut shcedule_map = self.sim_server.schedule_map.clone();
    for (_, queue) in &mut shcedule_map {
      if !queue.is_empty() {
        let mut is_schedule_rule = false;
        loop {
          if queue.is_empty() {
            break;
          }
          let rule_id = *queue.front().unwrap();
          let mut next_module = true;
          if let Some(rule_info) = self.sim_server.rules.get(rule_id) {
            if rule_info.is_always {
              if self.sim_server.is_guard_satisfied(rule_id) {
                //schedule always rule
                let func = self.sim_server.functions[rule_id];
                func(&mut self.sim_server, vec![]); // Call the function
                is_schedule_rule = true;
                break;
              } else {
                queue.pop_front();
                next_module = false;
                self.sim_server.always_rule_alive.insert(rule_id, true);
              }
            } else {
              if let Some(rule_info) = self.sim_server.method_top.get(&rule_id)
              {
                if let Some(&true) = self.sim_server.rule_run.get(rule_id) {
                  queue.pop_front();
                  next_module = false;
                } else {
                  let mut is_alive = false;
                  for top_always_rule_id in rule_info {
                    if let Some(&false) =
                      self.sim_server.always_rule_alive.get(top_always_rule_id)
                    {
                      is_alive = true;
                    }
                  }
                  if !is_alive && rule_info.len() > 0 {
                    queue.pop_front();
                    next_module = false;
                  }
                }
              }
            }
          }
          if next_module {
            break;
          }
        }
        if is_schedule_rule {
          break;
        }
      }
    }
    for (module_info, queue) in &mut self.sim_server.schedule_map {
      if !shcedule_map.contains_key(module_info) {
        shcedule_map.insert(module_info.clone(), queue.clone());
      }
    }
    self.sim_server.schedule_map = shcedule_map;
  }

  pub fn is_finish(&self) -> bool {
    for (_, queue) in self.sim_server.schedule_map.clone() {
      if !queue.is_empty() {
        return false;
      }
    }
    return true;
  }
}

fn main() {
  let mut sim_whole = SimWhole::new();
  let top_module = get_top_module();
  sim_whole.init_data_structure();
  for cycle in 1..=20 {
    println!("============ cycle{} start ============", cycle);
    sim_whole.sim_server.reset_server_status();
    sim_whole.start(top_module.clone());
    while !sim_whole.is_finish() {
      sim_whole.schedule();
    }
    println!("============ cycle{} end ============", cycle);
  }
}
