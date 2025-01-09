use indexmap::IndexSet;

use super::*;

#[derive(Clone)]
pub struct CppRuleRun {
  pub guards: Vec<CppExpr>,
  pub operations: Vec<CppExpr>,
  pub need_invoke: bool,
  pub full_path: String,
  pub inputs: Vec<String>,
  pub outputs: Vec<String>,
  pub invokes: Vec<String>,
}

impl CppRuleRun {
  pub fn create(
    rule_run: interface::RuleRun,
    belonged_module: String,
    path: String,
    rule_name: String,
    circuit: &ir::Circuit,
  ) -> anyhow::Result<Self> {
    let full_path = concat_path(&path, &rule_name);
    log::debug!("Creating rule run: {}", full_path);
    let mut guards = vec![];
    let mut operations = vec![];
    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut need_invoke = false;

    for guard_op in rule_run.guards() {
      match guard_op {
        interface::Guard::Explicit(guard) => {
          guards.push(CppExpr::eval(
            guard.clone(),
            circuit,
            &path,
            &belonged_module,
          )?);
        }
        interface::Guard::Invoke => {
          need_invoke = true;
        }
        interface::Guard::Implicit(status) => match status {
          interface::RunStatus::Run(inst_rule) => {
            guards.push(CppExpr::can_run(
              path.clone(),
              inst_rule.clone(),
              // circuit,
            )?);
          }
          interface::RunStatus::NotRun(inst_rule) => {
            guards.push(CppExpr::havnt_run(
              path.clone(),
              inst_rule.clone(),
              // circuit,
            )?);
          }
        },
      }
    }

    for op in rule_run.ops() {
      operations.push(CppExpr::eval(
        op.clone(),
        circuit,
        &path,
        &belonged_module,
      )?);
    }

    let module = circuit.module(&belonged_module).unwrap();
    let rule = module.rule_by_name(&rule_name).unwrap();
    for input in rule.inputs() {
      inputs.push(module.value_to_cpp(input));
    }
    for output in rule.outputs() {
      outputs.push(module.value_to_cpp(output));
    }

    let mut invokes = vec![];
    for guard in guards.iter().chain(operations.iter()) {
      match guard {
        CppExpr::Invoke(_, path, _) => {
          invokes.push(path.clone());
        }
        _ => {}
      }
    }

    Ok(Self {
      guards,
      operations,
      need_invoke,
      full_path,
      inputs,
      outputs,
      invokes,
    })
  }

  pub fn dump(&self) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}:\n", self.full_path));
    s.push_str(&format!(
      "  guards:-------\n{}\n",
      indent(
        self
          .guards
          .iter()
          .map(|g| g.to_cpp())
          .collect::<Vec<_>>()
          .join(""),
        4
      )
    ));
    s.push_str(&format!(
      "  operations:-------\n{}\n",
      indent(
        self
          .operations
          .iter()
          .map(|o| o.to_cpp())
          .collect::<Vec<_>>()
          .join(""),
        4
      )
    ));
    s
  }

  /// bool g_<rule_name>() {
  ///   ...
  /// }
  pub fn to_guard_cpp(&self, name_id_map: &IndexMap<String, usize>) -> String {
    let mut s = String::new();

    s.push_str(&format!("bool g_{}() {{\n", self.full_path));

    let mut body = String::new();
    let mut vars = vec![];

    for (idx, guard) in self.guards.iter().enumerate() {
      match guard {
        CppExpr::HavntRun(path) => {
          let var = format!("havnt_run_{}", idx);
          if !name_id_map.contains_key(path) {
            panic!("Path {} not found in name_id_map", path);
          }
          body.push_str(&format!(
            "bool {} = havnt_run({});\n",
            var, name_id_map[path]
          ));
          vars.push(var);
        }

        CppExpr::CanRun(path) => {
          let var = format!("can_run_{}", idx);
          body.push_str(&format!("bool {} = g_{}();\n", var, path));
          vars.push(var);
        }

        x @ _ => {
          body.push_str(&format!("{}", x.to_cpp()));
          vars.push(x.one_output());
        }
      }
    }

    // return x && y && ...
    body.push_str(&format!(
      "return {};\n",
      vars.into_iter().collect::<Vec<_>>().join(" && ")
    ));

    s.push_str(&format!("{}", indent(body, 2)));
    s.push_str("}\n");

    s
  }

  /// For always rule:
  /// void r_<rule_name>(<inputs>, <outputs>) {
  ///   // if is rule
  ///   bool guard = g_<rule_name>();
  ///   if (guard) {
  ///     ...
  ///   }
  /// }
  ///
  /// For method rule:
  /// void r_<rule_name>(<inputs>, <outputs>) {
  ///   // if is method
  ///   ...
  /// }
  pub fn to_rule_cpp(&self, name_id_map: &IndexMap<String, usize>) -> String {
    let mut s = String::new();
    // DATATYPE &i0, DATATYPE &i1, ...
    let inputs = self
      .inputs
      .iter()
      .map(|i| format!("{} &{}", i, i))
      .collect::<Vec<_>>();
    let outputs = self
      .outputs
      .iter()
      .map(|o| format!("{} &{}", o, o))
      .collect::<Vec<_>>();
    s.push_str(&format!(
      "void r_{}({}) {{\n",
      self.full_path,
      inputs
        .into_iter()
        .chain(outputs.into_iter())
        .collect::<Vec<_>>()
        .join(", ")
    ));

    let output_set = self.outputs.iter().collect::<IndexSet<_>>();

    let mut body = String::new();

    for op in self.operations.iter() {
      let op = op.to_cpp();
      // for line "DATATYPE o = xxx"; if o is in output_set, then remove
      // DATATYPE from the line
      let mut lines = op.split("\n").collect::<Vec<_>>();
      for line in lines.iter_mut() {
        if line.starts_with("DATATYPE") {
          let strips = line.split(" ").collect::<Vec<_>>();
          if output_set.contains(&strips[1].to_string()) {
            *line = &line[9..];
          }
        }
      }
      body.push_str(&lines.join("\n"));
    }

    let id = name_id_map[&self.full_path];
    // set_run(id);
    body.push_str(&format!("set_run({});", id));

    // current_relation.set(invoke_id, id, false);
    for invoke in self.invokes.iter() {
      if let Some(invoke_id) = name_id_map.get(invoke) {
        body.push_str(&format!(
          "current_relation.set({}, {}, false);\n",
          invoke_id, id
        ));
      }
    }

    if self.need_invoke {
      // is method rule
      s.push_str(&format!("{}", indent(body, 2)));
    } else {
      s.push_str(&format!("  bool guard = g_{}();\n", self.full_path));
      s.push_str(&format!("  if (guard) {{\n"));
      s.push_str(&format!("{}\n", indent(body, 4)));
      s.push_str("  }\n");
    }
    s.push_str("}\n");
    s
  }
}
