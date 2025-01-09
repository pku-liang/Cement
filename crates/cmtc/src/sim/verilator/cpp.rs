use super::*;
use pub_fields::pub_fields;

#[pub_fields]
pub struct VerilatorCpp {
  name: String,

  /// #include "Vadd2.h"
  includes: Vec<String>,
  /// V<module_name> <instance_name>; "Vadd2 *add2;"
  instances_decl: Vec<String>,
  /// <instance_name> = new V<module_name>{contextp};
  instances_construct: Vec<String>,
  /// <instance_name>->trace(m_trace, 1);
  instances_trace_on: Vec<String>,
  /// delete <instance_name>;
  instances_delete: Vec<String>,
  /// <instance_name>->clk = 0;
  /// <instance_name>->eval_step();
  /// m_trace->dump(sim_time++);
  /// <instance_name>->eval_end_step();
  /// <instance_name>->clk = 1;
  /// <instance_name>->eval_step();
  /// m_trace->dump(sim_time++);
  /// <instance_name>->eval_end_step();
  /// <instance_name>->feed_enable = 0;
  instances_tick: String,
  /// <instance_name>->rst = 1;
  /// tick();
  /// <instance_name>->rst = 0;
  /// tick();
  instances_reset: String,

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

impl VerilatorCpp {
  pub fn to_cpp(&self) -> String {
    let mut s = String::new();

    for include in self.includes.iter() {
      if !include.is_empty() {
        s.push_str(&format!("{}\n", include));
      }
    }

    s.push_str(&format!(
      "
#include \"verilated.h\"
#include \"verilated_vcd_c.h\"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <random>
#include <vector>
#include <list>

#define MAX_SIM_TIME 1000000000

const char *fname = \"{}.vcd\";

typedef vluint64_t DATATYPE;
vluint64_t sim_time = 0;


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

",
      self.name,
    ));

    s.push_str(&format!(
      "
class VerilatorRun {{
public:
  // instances
{}

  VerilatedContext *contextp;
  VerilatedVcdC *m_trace;
  std::mt19937_64 gen;
  bool exit;
  
  std::vector<std::list<int>> schedule_runs;
  std::vector<std::list<int>> current_runs;
  Matrix all_relation;
  Matrix current_relation;

  std::vector<bool> is_always;
  std::vector<bool> has_run;

  VerilatorRun(int argc, char **argv) {{
    contextp = new VerilatedContext();
    contextp->commandArgs(argc, argv);
    // instance init
{}
    Verilated::traceEverOn(true);
    m_trace = new VerilatedVcdC;
    // trace on
{}
    m_trace->open(fname);

    exit = false;
    schedule_runs = {};
    all_relation = std::vector<std::vector<int>>{};
    is_always = {};
  }}\n",
      indent(self.instances_decl.join("\n"), 2),
      indent(self.instances_construct.join("\n"), 4),
      indent(self.instances_trace_on.join("\n"), 4),
      self.schedule_runs,
      self.all_relations,
      self.is_always,
    ));

    s.push_str(&format!(
      "
  ~VerilatorRun() {{
    m_trace->close();
    delete m_trace;
    // instance delete
{}
    delete contextp;
  }}
    ",
      indent(self.instances_delete.join("\n"), 4),
    ));

    s.push_str(&format!(
      "
  void tick() {{
{}
  }}
    ",
      indent(self.instances_tick.clone(), 4),
    ));

    s.push_str(&format!(
      "
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


int main(int argc, char **argv) {{
  VerilatorRun run(argc, argv);
  run.loop();
  return 0;
}}

    ",
      indent(self.dispatch.clone(), 2),
      self.has_run,
      indent(self.rule_guards.join("\n"), 2),
      indent(self.rule_runs.join("\n"), 2),
      indent(self.instances_reset.clone(), 4),
    ));

    s
  }
}
