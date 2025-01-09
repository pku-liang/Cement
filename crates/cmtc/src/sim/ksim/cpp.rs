use super::instance::*;
use super::*;
use verilator::instance::InstanceTrait;
use verilator::{CppRuleRun, VerilatorRun};

use indexmap::IndexMap;

use cmtir::utils::indent;

pub struct KsimCpp {
  includes: Vec<String>,

  instances_reset: String,

  instances_decl: Vec<String>,

  instances_disable: String,

  /// bool g_<rule_name>() {
  ///   ...
  /// }
  rule_guards: Vec<String>,
  /// void r_<rule_name>() {
  ///   ...
  /// }
  rule_runs: Vec<String>,

  /// void dispatch(int id) {
  ///   switch (id) {
  ///     case 0:
  ///       r_do();
  ///       break;
  ///     ...
  ///   }
  /// }
  dispatch: String,

  /// {{schedule0}, {schedule1}, ...}
  schedule_runs: String,
  /// {{row0}, {row1}, ...}
  all_relations: String,
  /// {true, false, ...}
  is_always: String,
  /// {false, false, ...}
  has_run: String,
}

impl KsimCpp {
  pub fn to_cpp(&self) -> String {
    let mut s = String::new();

    for include in self.includes.iter() {
      if !include.is_empty() {
        s.push_str(&format!("{}\n", include));
      }
    }

    s.push_str(&format!(
      "
#include<cstdlib>
#include<cstdio>
#include<iostream>
#include<vector>
#include<list>

typedef uint64_t DATATYPE;


#define MAX_SIM_TIME 1000000000
uint64_t sim_time = 0;

class Matrix {{
  private:
    std::vector<std::vector<bool>> data;
    std::vector<int> row_ones;
    int size;
  
  public:
    Matrix() : size(0) {{}}
  
    Matrix(const std::vector<std::vector<int>>& input) {{
      size = input.size();
      data.resize(size);
      row_ones.resize(size, 0);
      for(int i = 0; i < size; i++) {{
        data[i].resize(size);
        for(int j = 0; j < size; j++) {{
          data[i][j] = input[i][j] != 0;
          if(data[i][j]) {{
            row_ones[i]++;
          }}
        }}
      }}
    }}
    
    void set(int i, int j, bool value) {{
      if (i >= 0 && i < size && j >= 0 && j < size) {{
        if (data[i][j] != value) {{
          data[i][j] = value;
          row_ones[i] += value ? 1 : -1;
        }}
      }}
    }}
  
    bool get(int i, int j) const {{
      if (i >= 0 && i < size && j >= 0 && j < size) {{
        return data[i][j];
      }}
      return false;
    }}
  
    bool is_all_zero(int i) const {{
      return row_ones[i] == 0;
    }}
}};

class KsimRun {{
public:
  bool exit;
  std::vector<std::list<int>> schedule_runs;
  std::vector<std::list<int>> current_runs;
  Matrix all_relation;
  Matrix current_relation;

  std::vector<bool> is_always;
  std::vector<bool> has_run;

  // instance decls
{}

  KsimRun() {{
    exit = false;
    schedule_runs = {};
    all_relation = std::vector<std::vector<int>>{};
    is_always = {};
  }}

      ",
      indent(self.instances_decl.join("\n"), 4),
      self.schedule_runs,
      self.all_relations,
      self.is_always,
    ));

    s.push_str(&format!(
      "
  void tick() {{
    eval();
{}
  }}

{}

  void setup() {{
    current_runs = schedule_runs;
    current_relation = all_relation;
    has_run = {};
  }}

  bool havnt_run(int id) {{
    return !has_run[id];
  }}

  void set_run(int id) {{
    has_run[id] = true;
  }}

  int next_run() {{
    for (auto& schedule : current_runs) {{
      while (!schedule.empty() && current_relation.is_all_zero(schedule.front())) {{
        auto run_id = schedule.front();
        if (is_always[run_id]) {{
          schedule.pop_front();
          return run_id;
        }} else {{
          schedule.pop_front();
        }}
      }}
    }}
    return -1;
  }}

  // guards
{}

  // rules
{}

  void reset() {{
{}  
  }}

  void set_exit() {{
    exit = true;
  }}

  void loop() {{
    reset();
    bool end = false;
    while (!end && !exit && sim_time < MAX_SIM_TIME) {{
      tick();
      setup();
      int run_id;
      while ((run_id = next_run()) != -1) {{
        dispatch(run_id);
      }}
    }}
  }}
}};

int main(int argc, char ** argv) {{
  KsimRun run;
  run.loop();
  return 0;
}}

    ",
    indent(self.instances_disable.clone(), 4),
    indent(self.dispatch.clone(), 4),
    indent(self.has_run.clone(), 4),
    indent(self.rule_guards.join("\n"), 4),
    indent(self.rule_runs.join("\n"), 4),
    indent(self.instances_reset.clone(), 4),
    ));

    s
  }
}

fn dispatch_in_cpp(name_id_map: &IndexMap<String, usize>) -> String {
  let mut swithes = String::new();

  for (name, id) in name_id_map.iter() {
    swithes.push_str(&format!("case {}:\n", id));
    swithes.push_str(&format!("  r_{}();\n", name));
    swithes.push_str("  break;\n");
  }

  swithes.push_str("  default:\n");
  swithes.push_str("    break;\n");

  format!(
    "void dispatch(int id) {{\n  switch (id) {{\n{}\n    }}\n  }}\n",
    indent(swithes, 4)
  )
}

fn schedule_runs_in_cpp(
  schedule_runs: &Vec<verilator::ScheduleRun>,
  name_id_map: &IndexMap<String, usize>,
) -> String {
  let mut s = String::new();
  s.push_str("{");
  for schedule in schedule_runs.iter() {
    s.push_str(&format!(
      "{{{}}}",
      schedule
        .rules
        .iter()
        .map(|r| name_id_map[&r.full_path].to_string())
        .collect::<Vec<_>>()
        .join(", ")
    ));
    s.push_str(",");
  }
  s.push_str("}");
  s
}

fn all_relations_in_cpp(invoke_relations: &Vec<Vec<bool>>) -> String {
  let mut s = String::new();
  s.push_str("{");
  for row in invoke_relations.iter() {
    s.push_str(&format!(
      "{{{}}}",
      row
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ")
    ));
    s.push_str(",");
  }
  s.push_str("}");
  s
}

fn is_always_in_cpp(is_always: &Vec<bool>) -> String {
  let mut s = String::new();
  s.push_str("{");
  s.push_str(&format!(
    "{}",
    is_always
      .iter()
      .map(|b| b.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  ));
  s.push_str("}");
  s
}

impl VerilatorRun {
  pub fn build_ksim(&self) -> anyhow::Result<KsimCpp> {
    // eprintln!("name_id_map: {:#?}", self.name_id_map);

    let instance_impls: Vec<InstanceImpl> = self
      .instances
      .iter()
      .map(|i| InstanceImpl::from_instance(&self.circuit, i))
      .collect();

    // eprintln!(
    //   "instances: {:?}",
    //   self
    //     .instances
    //     .iter()
    //     .map(|i| i.module_name.clone())
    //     .collect::<Vec<_>>()
    //     .join(", ")
    // );

    let includes = instance_impls.iter().map(|i| i.to_include()).collect();

    let instances_decl = instance_impls.iter().map(|i| i.to_decl()).collect();

    let instances_reset = reset_in_cpp(&instance_impls);

    let instances_disable = disable_in_cpp(&instance_impls);

    let cpp_rule_runs: Vec<CppRuleRun> = self
      .schedule_runs
      .iter()
      .map(|s| s.rules.clone())
      .flatten()
      .collect();

    let rule_guards = cpp_rule_runs
      .iter()
      .map(|r| r.to_guard_cpp(&self.name_id_map))
      .chain(instance_impls.iter().map(|i| i.to_guard_cpp()))
      .collect();
    let rule_runs = cpp_rule_runs
      .iter()
      .map(|r| r.to_rule_cpp(&self.name_id_map))
      .chain(instance_impls.iter().map(|i| i.to_rule_cpp()))
      .collect();

    let dispatch = dispatch_in_cpp(&self.name_id_map);

    let schedule_runs =
      schedule_runs_in_cpp(&self.schedule_runs, &self.name_id_map);

    let all_relations = all_relations_in_cpp(&self.invoke_relations);
    let is_always = is_always_in_cpp(&self.is_always);
    let has_run = format!(
      "{{{}}}",
      self
        .is_always
        .iter()
        .map(|_| "false".to_string())
        .collect::<Vec<_>>()
        .join(", ")
    );

    Ok(KsimCpp {
      includes,
      instances_decl,
      instances_reset,
      instances_disable,
      rule_guards,
      rule_runs,
      dispatch,
      schedule_runs,
      all_relations,
      is_always,
      has_run,
    })
  }
}
