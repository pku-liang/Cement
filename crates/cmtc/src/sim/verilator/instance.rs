use slotmap::SecondaryMap;

use super::*;

impl Instance {
  pub fn to_impl(&self, circuit: &ir::Circuit) -> InstanceImpl {
    let module = circuit
      .module(&self.module_name)
      .expect(&format!("Module {} not found", self.module_name));
    if module.is_virtual() {
      let module_name = self.module_name.as_str().to_lowercase();
      match module_name.as_str() {
        "integer" => InstanceImpl::Integer(IntegerInstance {
          instance_name: self.full_path.clone(),
        }),
        _ => panic!("Unsupported virtual module: {}", self.module_name),
      }
    } else {
      let mut value_id_to_name = SecondaryMap::new();
      if module.is_native() {
        for value in module.inputs().chain(module.outputs()) {
          value_id_to_name.insert(value, module.value_to_cpp(value));
        }
      } else {
        for (value, name) in module
          .inputs_with_binding()
          .chain(module.outputs_with_binding())
        {
          value_id_to_name.insert(value, name);
        }
      }
      InstanceImpl::Sv(SvInstance {
        instance_name: self.full_path.clone(),
        module_name: self.module_name.clone(),
        // inputs: module.filter_port_names(module.inputs_w_flip().into_iter()),
        // outputs: module.filter_port_names(module.outputs_w_flip().
        // into_iter()),
        methods: {
          module
            .method_rules()
            .filter_map(|r| {
              if r.is_private() {
                None
              } else {
                let name = r.name.clone();
                let enable = if module.is_native() {
                  Some(format!("{}_enable", name))
                } else {
                  r.enable.clone()
                };
                let ready = if module.is_native() {
                  Some(format!("{}_ready", name))
                } else {
                  r.ready.clone()
                };
                let inputs = r
                  .inputs()
                  .iter()
                  .map(|i| value_id_to_name[*i].clone())
                  .collect();
                let outputs = r
                  .outputs()
                  .iter()
                  .map(|i| value_id_to_name[*i].clone())
                  .collect();
                Some(SvMethod {
                  name,
                  enable,
                  ready,
                  inputs,
                  outputs,
                })
              }
            })
            .collect()
        },
      })
    }
  }
}

pub trait InstanceTrait {
  fn to_include(&self) -> String;
  fn to_decl(&self) -> String;
  fn to_construct(&self) -> String;
  fn to_trace_on(&self) -> String;
  fn to_delete(&self) -> String;
  fn to_clk(&self, clk: u8) -> String;
  fn to_eval_end_step(&self) -> String;
  fn to_disable(&self) -> String;
  fn to_rst(&self, rst: u8) -> String;
  fn to_guard_cpp(&self) -> String;
  fn to_rule_cpp(&self) -> String;
  fn to_cmake(&self) -> String;
}

pub struct SvMethod {
  name: String,
  enable: Option<String>,
  ready: Option<String>,
  inputs: Vec<String>,
  outputs: Vec<String>,
}
pub struct SvInstance {
  instance_name: String,
  module_name: String,
  // inputs: Vec<String>,
  // outputs: Vec<String>,
  methods: Vec<SvMethod>,
}

impl InstanceTrait for SvInstance {
  fn to_include(&self) -> String {
    format!("#include \"V{}.h\"", self.module_name)
  }

  fn to_decl(&self) -> String {
    format!("V{} *{};", self.module_name, self.instance_name)
  }

  fn to_construct(&self) -> String {
    format!(
      "{} = new V{} {{contextp}};",
      self.instance_name, self.module_name
    )
  }

  fn to_trace_on(&self) -> String {
    format!("{}->trace(m_trace, 1);", self.instance_name)
  }

  fn to_delete(&self) -> String {
    format!("delete {};", self.instance_name)
  }

  fn to_clk(&self, clk: u8) -> String {
    format!(
      "{}->clk = {};\n{}->eval_step();\nm_trace->dump(sim_time++);\n",
      self.instance_name, clk, self.instance_name
    )
  }

  fn to_eval_end_step(&self) -> String {
    format!("{}->eval_end_step();\n", self.instance_name)
  }

  fn to_disable(&self) -> String {
    let mut s = String::new();
    for SvMethod { enable, .. } in self.methods.iter() {
      if let Some(enable) = enable {
        s.push_str(&format!("{}->{} = 0;\n", self.instance_name, enable));
      }
    }
    s
  }

  fn to_rst(&self, rst: u8) -> String {
    format!("{}->rst = {};\n", self.instance_name, rst)
  }

  /// bool g_<instance_name>_<method_name>() {
  ///   return <instance_name>-><method_name>_ready;
  /// }
  fn to_guard_cpp(&self) -> String {
    let mut s = String::new();
    for SvMethod {
      name: method_name,
      ready,
      ..
    } in self.methods.iter()
    {
      if let Some(ready) = ready {
        s.push_str(&format!(
          "bool g_{}_{}() {{ return {}->{};}}\n",
          self.instance_name, method_name, self.instance_name, ready
        ));
      } else {
        s.push_str(&format!(
          "bool g_{}_{}() {{ return true; }}\n",
          self.instance_name, method_name
        ));
      }
    }
    s
  }

  /// void r_<instance_name>_<method_name>(<inputs><outputs>) {
  ///   <instance_name>-><inputs> = <inputs>;
  ///   <instance_name>->enable = 1;
  ///   <instance_name>->eval();
  ///   <outputs> = <instance_name>-><outputs>;
  /// }
  fn to_rule_cpp(&self) -> String {
    let mut s = String::new();

    for SvMethod {
      name: method_name,
      inputs,
      outputs,
      enable,
      ..
    } in self.methods.iter()
    {
      // DATATYPE &i0, DATATYPE &i1, ...
      let inputs_list = inputs
        .iter()
        .map(|i| format!("DATATYPE &{}", i))
        .collect::<Vec<_>>();
      let outputs_list = outputs
        .iter()
        .map(|i| format!("DATATYPE &{}", i))
        .collect::<Vec<_>>();
      s.push_str(&format!(
        "void r_{}_{}({}) {{\n",
        self.instance_name,
        method_name,
        inputs_list
          .iter()
          .cloned()
          .chain(outputs_list.iter().cloned())
          .collect::<Vec<_>>()
          .join(", ")
      ));
      for input in inputs {
        s.push_str(&format!(
          "  {}->{} = {};\n",
          self.instance_name, input, input
        ));
      }
      if let Some(enable) = enable {
        s.push_str(&format!("  {}->{} = 1;\n", self.instance_name, enable));
      }
      s.push_str(&format!("  {}->eval();\n", self.instance_name));
      for output in outputs {
        s.push_str(&format!(
          "  {} = {}->{};\n",
          output, self.instance_name, output
        ));
      }
      s.push_str("}\n");
    }

    s
  }

  fn to_cmake(&self) -> String {
    format!(
      "
verilate(Vsim
SOURCES ${{DESIGNDIR}}/{}.sv
TOP_MODULE {}
TRACE
)
  ",
      self.module_name, self.module_name
    )
  }
}

pub struct IntegerInstance {
  instance_name: String,
}

impl InstanceTrait for IntegerInstance {
  fn to_include(&self) -> String {
    format!("")
  }

  fn to_decl(&self) -> String {
    format!("int {};", self.instance_name)
  }

  fn to_construct(&self) -> String {
    format!("{} = 0;", self.instance_name)
  }

  fn to_trace_on(&self) -> String {
    "".to_string()
  }

  fn to_delete(&self) -> String {
    "".to_string()
  }

  fn to_clk(&self, _clk: u8) -> String {
    "".to_string()
  }

  fn to_eval_end_step(&self) -> String {
    "".to_string()
  }

  fn to_disable(&self) -> String {
    "".to_string()
  }

  fn to_rst(&self, _rst: u8) -> String {
    "".to_string()
  }

  /// bool g_<instance_name>_read() {
  ///   return true;
  /// }
  /// bool g_<instance_name>_write() {
  ///   return true;
  /// }
  fn to_guard_cpp(&self) -> String {
    format!("bool g_{}_read() {{ return true; }}\n", self.instance_name)
      + &format!("bool g_{}_write() {{ return true; }}\n", self.instance_name)
  }

  /// void r_<instance_name>_read(DATATYPE &x) {
  ///   x = <instance_name>;
  /// }
  /// void r_<instance_name>_write(DATATYPE x) {
  ///   <instance_name> = x;
  /// }
  fn to_rule_cpp(&self) -> String {
    format!(
      "void r_{}_read(DATATYPE &x) {{ x = {}; }}\n",
      self.instance_name, self.instance_name
    ) + &format!(
      "void r_{}_write(DATATYPE x) {{ {} = x; }}\n",
      self.instance_name, self.instance_name
    )
  }

  fn to_cmake(&self) -> String {
    "".to_string()
  }
}

pub enum InstanceImpl {
  Sv(SvInstance),
  Integer(IntegerInstance),
}

impl InstanceTrait for InstanceImpl {
  fn to_include(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_include(),
      InstanceImpl::Integer(i) => i.to_include(),
    }
  }

  fn to_decl(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_decl(),
      InstanceImpl::Integer(i) => i.to_decl(),
    }
  }

  fn to_construct(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_construct(),
      InstanceImpl::Integer(i) => i.to_construct(),
    }
  }

  fn to_trace_on(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_trace_on(),
      InstanceImpl::Integer(i) => i.to_trace_on(),
    }
  }

  fn to_delete(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_delete(),
      InstanceImpl::Integer(i) => i.to_delete(),
    }
  }

  fn to_clk(&self, clk: u8) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_clk(clk),
      InstanceImpl::Integer(i) => i.to_clk(clk),
    }
  }

  fn to_eval_end_step(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_eval_end_step(),
      InstanceImpl::Integer(i) => i.to_eval_end_step(),
    }
  }

  fn to_disable(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_disable(),
      InstanceImpl::Integer(i) => i.to_disable(),
    }
  }

  fn to_rst(&self, rst: u8) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_rst(rst),
      InstanceImpl::Integer(i) => i.to_rst(rst),
    }
  }

  fn to_guard_cpp(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_guard_cpp(),
      InstanceImpl::Integer(i) => i.to_guard_cpp(),
    }
  }

  fn to_rule_cpp(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_rule_cpp(),
      InstanceImpl::Integer(i) => i.to_rule_cpp(),
    }
  }

  fn to_cmake(&self) -> String {
    match self {
      InstanceImpl::Sv(i) => i.to_cmake(),
      InstanceImpl::Integer(i) => i.to_cmake(),
    }
  }
}

pub fn tick_in_cpp(instance_impls: &Vec<InstanceImpl>) -> String {
  let mut s = String::new();
  for clk in 0..2 {
    for instance in instance_impls {
      s.push_str(&instance.to_clk(clk));
    }
    for instance in instance_impls {
      s.push_str(&instance.to_eval_end_step());
    }
    // s.push_str("sim_time++;\n");
  }

  for instance in instance_impls {
    s.push_str(&instance.to_disable());
  }

  s
}

pub fn reset_in_cpp(instance_impls: &Vec<InstanceImpl>) -> String {
  let mut s = String::new();
  for rst in (0..2).rev() {
    for instance in instance_impls {
      s.push_str(&instance.to_rst(rst));
    }
    s.push_str("tick();\n");
  }
  s
}
