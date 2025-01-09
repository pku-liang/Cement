//! IR structure (module, function, circuit).

pub use super::*;
pub use json;

/// io bindings for external modules.
#[derive(Default, Debug, Clone, SExpr)]
pub struct InOutBindings {
  pub inputs: Vec<String>,
  pub outputs: Vec<String>,
}

/// FIRRTL string in external modules.
#[derive(Default, Debug, Clone)]
pub struct FirString {
  pub inner: String,
}

impl Parse for FirString {
  fn parse(p: &mut Parser) -> Result<Self, String> {
    Ok(FirString {
      inner: p.expect(Token::MultiLineStr)?.to_string(),
    })
  }
}

impl Print for FirString {
  fn print(&self, p: &mut Printer) {
    write!(p, "\n{}", self.inner);
  }
}

/// Body of external modules: bindings, clock, reset, and FIRRTL string.
#[derive(Default, Debug, Clone, SExpr)]
pub struct ExtModuleBody {
  #[pp(surrounded = "bindings")]
  pub bindings: InOutBindings,
  #[pp(surrounded = "clock")]
  pub clock: Option<String>,
  #[pp(surrounded = "reset")]
  pub reset: Option<String>,
  #[pp(surrounded = "fir")]
  pub fir_str: FirString,
}

impl ExtModuleBody {
  /// Create an `ExtModuleBody` with bindings, clock, reset, and FIRRTL string.
  pub fn new(
    inputs: Vec<String>,
    outputs: Vec<String>,
    clock: Option<String>,
    reset: Option<String>,
    fir_str: String,
  ) -> Self {
    ExtModuleBody {
      bindings: InOutBindings { inputs, outputs },
      clock,
      reset,
      fir_str: FirString { inner: fir_str },
    }
  }
  /// Create an `ExtModuleBody` with only FIRRTL string.
  pub fn with_firrtl(fir_str: String) -> Self {
    ExtModuleBody {
      fir_str: FirString { inner: fir_str },
      ..Default::default()
    }
  }
}

/// Body of native modules: instances.
#[derive(Default, Debug, Clone, SExpr)]
pub struct NativeModuleBody {
  #[pp(list_ml = "instances")]
  pub instances: Vec<InstDef>,
}

/// Body of modules: external modules or native modules or placeholders (for
/// virtual modules).
#[derive(Debug, Clone, SExpr)]
pub enum ModuleBody {
  #[pp(surrounded)]
  #[pp(indent = 1)]
  Ext(ExtModuleBody),
  Native(NativeModuleBody),
  PlaceHolder,
}

impl Default for ModuleBody {
  fn default() -> Self {
    ModuleBody::Native(NativeModuleBody { instances: vec![] })
  }
}

impl ModuleBody {
  /// Get the native module body or panic if it is not a native module.
  pub fn as_native(&self) -> &NativeModuleBody {
    match self {
      ModuleBody::Native(body) => body,
      _ => panic!("ModuleBody is not native"),
    }
  }
  /// Get the native module body or panic if it is not a native module.
  pub fn as_native_mut(&mut self) -> &mut NativeModuleBody {
    match self {
      ModuleBody::Native(body) => body,
      _ => panic!("ModuleBody is not native"),
    }
  }
  /// Get the external module body or panic if it is not an external module.
  pub fn as_ext(&self) -> &ExtModuleBody {
    match self {
      ModuleBody::Ext(body) => body,
      _ => panic!("ModuleBody is not ext"),
    }
  }
  /// Get the external module body or panic if it is not an external module.
  pub fn as_ext_mut(&mut self) -> &mut ExtModuleBody {
    match self {
      ModuleBody::Ext(body) => body,
      _ => panic!("ModuleBody is not ext"),
    }
  }
}

/// A module in the IR.
/// (module "add2_2stage" (
///   (annotations <json>)
///   (inputs (<list of value ids>))
///   (outputs (<list of value ids>))
///   (wires (<list of value ids>))
///   (native/ext/placeholder <body>)
///   (rules
///     (<rule>)*
///   )
///   (rule_rels
///     (<rule_rel>)*
///   )
/// ))
#[derive(Debug, Clone, SExpr)]
pub struct Module {
  #[pp(open)]
  #[pp(kw = "module")]
  pub name: String,
  #[pp(map)]
  /// A map from value id to value (name, type).
  pub values: SlotMap<ValueId, Value>,
  #[pp(open = 1)]
  #[pp(surrounded = "annotations")]
  pub annotations: json::object::Object,
  #[pp(nl)]
  #[pp(surrounded = "inputs")]
  pub inputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "outputs")]
  pub outputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "wires")]
  pub wires: Vec<ValueId>,
  #[pp(nl)]
  pub body: ModuleBody,
  #[pp(nl)]
  #[pp(list_ml = "rules")]
  pub rules: Vec<Rule>,
  #[pp(nl)]
  #[pp(list_ml = "rule_rels")]
  #[pp(close = -1)]
  #[pp(close)]
  pub rule_rels: Vec<RuleRel>,
}

impl Module {
  /// Get the name of the module.
  pub fn name(&self) -> &str {
    &self.name
  }
  /// Create a new module with the given name.
  pub fn with_name(name: String) -> Self {
    Module {
      name,
      values: ValueMap::with_key(),
      annotations: json::object::Object::new(),
      inputs: vec![],
      outputs: vec![],
      wires: vec![],
      body: Default::default(),
      rule_rels: vec![],
      rules: vec![],
    }
  }
  /// Get the spans of the module from annotations.
  pub fn spans(&self) -> impl Iterator<Item = MySpan> + '_ {
    self.annotations.iter().filter_map(|(k, v)| {
      if k.ends_with("span") {
        MySpan::from_json(v)
      } else {
        None
      }
    })
  }
  /// Check if the module has an annotation with the given key and value is true.
  pub fn annotation_is_true(&self, key: &str) -> bool {
    // log::info!("module {}'s annotation has key {}? {}", self.name, key,
    // self.annotations.get(key).is_some());
    self.annotations.get(key).map_or(false, |v| {
      // convert v to bool
      // log::info!("value is {}", v.to_string());
      match v {
        json::JsonValue::Boolean(b) => *b,
        json::JsonValue::Short(n) => {
          // log::info!("value {} == true? {}", n, n.as_str() == "true");
          n.as_str() == "true"
        }
        json::JsonValue::String(s) => {
          // log::info!("value {} == true? {}", s, s == "true");
          s == "true"
        }
        _ => false,
      }
    })
  }
  /// Check if the module has an annotation with the given key and value is false.
  pub fn annotation_is_false(&self, key: &str) -> bool {
    !self.annotation_is_true(key)
  }
  /// Set the module as top module.
  pub fn as_top(mut self) -> Self {
    self.add_annotation("is_top".to_string(), "true".to_string());
    self
  }
  /// Set the module as top module. 
  pub fn set_top(&mut self) {
    self.add_annotation("is_top".to_string(), "true".to_string());
  }
  /// Unset the module as top module.
  pub fn unset_top(&mut self) {
    self.annotations.remove("is_top");
  }
  /// Check if the module is top module.
  pub fn is_top(&self) -> bool {
    self.annotation_is_true("is_top")
  }
  /// Check if the module is testbench.
  pub fn is_tb(&self) -> bool {
    self.annotation_is_true("is_tb")
  }
  /// Set the module as testbench.
  pub fn set_tb(&mut self) {
    self.add_annotation("is_tb".to_string(), "true".to_string());
  }
  /// Unset the module as testbench.
  pub fn unset_tb(&mut self) {
    self.annotations.remove("is_tb");
  }
  /// Set the module as synthesized.
  pub fn set_synthesis(&mut self) {
    self.add_annotation("synthesis".to_string(), "true".to_string());
  }
  /// Check if the module is synthesized.
  pub fn to_be_synthesize(&self) -> bool {
    !self.is_tb()
      && !self.is_virtual()
      && (self.is_top()
        || self.is_ext()
        || self.annotation_is_true("synthesis"))
  }

  /// Get all instances in the module.
  pub fn instances(&self) -> Vec<&InstDef> {
    match &self.body {
      ModuleBody::Native(body) => body.instances.iter().collect(),
      _ => vec![],
    }
  }
  /// Get the instance definition by name.
  pub fn instance_by_name(&self, name: &str) -> Option<&InstDef> {
    self
      .body
      .as_native()
      .instances
      .iter()
      .find(|inst| inst.name == name)
  }
  /// Get the instance definition (mut) by name.
  pub fn instance_by_name_mut(&mut self, name: &str) -> Option<&mut InstDef> {
    self
      .body
      .as_native_mut()
      .instances
      .iter_mut()
      .find(|inst| inst.name == name)
  }
  /// Get all instances in the module (mut).
  pub fn instances_mut(&mut self) -> &mut Vec<InstDef> {
    self.body.as_native_mut().instances.as_mut()
  }
  /// Add an instance to the module.
  pub fn add_instance(&mut self, name: String, module: String) {
    self
      .body
      .as_native_mut()
      .instances
      .push(InstDef { name, module });
  }
  /// Add a value to the module.
  pub fn add_value(
    &mut self,
    name: Option<String>,
    typ: Option<Type>,
  ) -> ValueId {
    self.values.insert(Value { ty: typ, name })
  }
  /// Add an annotation (key, value) to the module.
  pub fn add_annotation(&mut self, key: String, value: String) {
    self
      .annotations
      .insert(&key, json::JsonValue::Boolean(value == "true"));
  }
  /// Get all rules in the module (mut).
  pub fn rules_mut(&mut self) -> &mut Vec<Rule> {
    self.rules.as_mut()
  }
  /// Get all rules in the module.
  pub fn rules(&self) -> impl Iterator<Item = &Rule> {
    self.rules.iter()
  }
  /// Get all always rules in the module.
  pub fn always_rules(&self) -> impl Iterator<Item = &Rule> {
    self.rules.iter().filter(|r| r.is_always())
  }
  /// Get all method rules in the module.
  pub fn method_rules(&self) -> impl Iterator<Item = &Rule> {
    self.rules.iter().filter(|r| r.is_method())
  }
  /// Get the rule by name.
  pub fn rule_by_name(&self, name: &str) -> Option<&Rule> {
    self.rules.iter().find(|r| r.name == name)
  }
  /// Add a rule to the module.
  pub fn add_rule(&mut self, rule: Rule) {
    self.rules.push(rule);
  }
  /// Get all rule relations in the module (mut).
  pub fn rule_rels_mut(&mut self) -> &mut Vec<RuleRel> {
    self.rule_rels.as_mut()
  }
  /// Get the type of the value by id.
  pub fn value_typ(&self, id: ValueId) -> Option<&Type> {
    self.values.get(id).and_then(|v| v.ty.as_ref())
  }
  /// Add a rule relation to the module.
  pub fn add_rule_rel(&mut self, rule_rel: RuleRel) {
    self.rule_rels.push(rule_rel);
  }
  /// Add an input to the module.
  pub fn add_input(&mut self, id: ValueId) {
    self.inputs.push(id);
  }
  /// Add an output to the module.
  pub fn add_output(&mut self, id: ValueId) {
    self.outputs.push(id);
  }
  /// Add a wire to the module.
  pub fn add_wire(&mut self, id: ValueId) {
    self.wires.push(id);
  }
  /// Add FIRRTL string to the module.
  pub fn add_firrtl(&mut self, firrtl: String) {
    self.body.as_ext_mut().fir_str.inner = firrtl;
  }
  /// Get the FIRRTL string of the module.
  pub fn firrtl(&self) -> String {
    self.body.as_ext().fir_str.inner.clone()
  }
  /// Set the module as external module.
  pub fn with_ext_body(&mut self, body: ExtModuleBody) {
    self.body = ModuleBody::Ext(body);
  }
  /// Set the module as placeholder.
  pub fn with_placeholder(&mut self) {
    self.body = ModuleBody::PlaceHolder;
  }
  /// Set the module as default external module body.
  pub fn with_default_ext_body(&mut self) {
    let body = ExtModuleBody::default();
    self.with_ext_body(body)
  }
  /// Add a bind input to the module.
  pub fn add_bind_input(&mut self, input: String) {
    self.body.as_ext_mut().bindings.inputs.push(input);
  }
  /// Get all inputs with binding.
  pub fn inputs_with_binding(
    &self,
  ) -> impl Iterator<Item = (ValueId, String)> + use<'_> {
    self
      .inputs
      .iter()
      .cloned()
      .zip(self.body.as_ext().bindings.inputs.iter().cloned())
  }
  /// Add a bind output to the module.
  pub fn add_bind_output(&mut self, output: String) {
    self.body.as_ext_mut().bindings.outputs.push(output);
  }
  /// Get all outputs with binding.
  pub fn outputs_with_binding(
    &self,
  ) -> impl Iterator<Item = (ValueId, String)> + use<'_> {
    self
      .outputs
      .iter()
      .cloned()
      .zip(self.body.as_ext().bindings.outputs.iter().cloned())
  }

  /// Add a bind clock to the module. 
  pub fn add_bind_clock(&mut self, clock: String) {
    self.body.as_ext_mut().clock = Some(clock);
  }
  /// Add an external rule to the module.
  pub fn add_ext_rule(&mut self, rule: Rule) {
    assert!((self.is_ext() || self.is_virtual()) && rule.is_ext());
    self.rules.push(rule);
  }
  /// Check if the module is external module.
  pub fn is_ext(&self) -> bool {
    matches!(self.body, ModuleBody::Ext(_))
  }
  /// Check if the module is virtual module.
  pub fn is_virtual(&self) -> bool {
    matches!(self.body, ModuleBody::PlaceHolder)
      || self.annotation_is_true("is_virtual")
  }
  /// Check if the module is native module.
  pub fn is_native(&self) -> bool {
    matches!(self.body, ModuleBody::Native(_))
  }
  /// Get all rule relations in the module.
  pub fn rule_rels(&self) -> impl Iterator<Item = &RuleRel> {
    self.rule_rels.iter()
  }
  /// Clear all rule relations in the module.
  pub fn clear_rule_sched_orders(&mut self) {
    self
      .rule_rels
      .retain(|rel| !matches!(rel, RuleRel::Schedule { .. }));
  }
  /// Add a schedule to the module.
  pub fn add_schedule(&mut self, rules: Vec<InstRule>) {
    self.rule_rels.push(RuleRel::Schedule(rules));
  }
  /// Get all schedules in the module.
  pub fn schedules(&self) -> impl Iterator<Item = Vec<InstRule>> + '_ {
    self.rule_rels.iter().filter_map(|rel| {
      if let RuleRel::Schedule(rules) = rel {
        Some(rules.iter().map(|r| r.canonicalize()).collect())
      } else {
        None
      }
    })
  }
  /// Add a method relation to the module.
  pub fn add_method_rel(
    &mut self,
    rel: MethodRel,
    lhs: Vec<InstRule>,
    rhs: Vec<InstRule>,
  ) {
    self.rule_rels.push(RuleRel::Method { rel, lhs, rhs });
  }
  /// Get all method relations in the module.
  pub fn method_rels(&self) -> impl Iterator<Item = &RuleRel> {
    self
      .rule_rels
      .iter()
      .filter(|rel| matches!(rel, RuleRel::Method { .. }))
  }
  /// Check if the module has an input by id.
  pub fn has_input(&self, id: ValueId) -> bool {
    self.inputs.contains(&id)
  }
  /// Get all inputs in the module.
  pub fn inputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    self.inputs.iter().cloned()
  }
  /// Check if the module has an output by id.
  pub fn has_output(&self, id: ValueId) -> bool {
    self.outputs.contains(&id)
  }
  /// Get all outputs in the module.
  pub fn outputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    self.outputs.iter().cloned()
  }
  /// Check if the module has a wire by id.
  pub fn wires(&self) -> impl Iterator<Item = ValueId> + '_ {
    self.wires.iter().cloned()
  }
  /// Clone a value and add it to the module.
  pub fn clone_value(&mut self, id: ValueId, suffix: &str) -> ValueId {
    if let Some(value) = self.values.get(id) {
      self.add_value(
        value.name.clone().map(|n| format!("{}_{}", n, suffix)),
        value.ty.clone(),
      )
    } else {
      panic!("value {} not found", id);
    }
  }
  /// Get the external module body of the module.
  pub fn ext_body(&self) -> Option<ExtModuleBody> {
    if self.is_ext() {
      Some(self.body.as_ext().clone())
    } else {
      None
    }
  }
}

/// Function in `cmtir`. 
/// **Not supported in `cmtrs` frontend yet.**
#[derive(Default, Debug, Clone, SExpr)]
pub struct Function {
  #[pp(open)]
  #[pp(kw = "function")]
  pub name: String,
  #[pp(map)]
  pub values: SlotMap<ValueId, Value>,
  #[pp(open = 1)]
  #[pp(surrounded = "inputs")]
  pub inputs: Vec<ValueId>,
  #[pp(nl)]
  #[pp(surrounded = "outputs")]
  pub outputs: Vec<Option<Type>>,
  #[pp(nl)]
  #[pp(list_ml = "instances")]
  pub instances: Vec<InstDef>,
  #[pp(nl)]
  #[pp(list_ml)]
  #[pp(close = -1)]
  #[pp(close)]
  pub ops: Vec<Op>,
}

impl Function {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn with_name(name: String) -> Self {
    Function {
      name,
      values: ValueMap::with_key(),
      inputs: vec![],
      instances: vec![],
      outputs: vec![],
      ops: vec![],
    }
  }

  pub fn add_value(
    &mut self,
    name: Option<String>,
    typ: Option<Type>,
  ) -> ValueId {
    self.values.insert(Value { ty: typ, name })
  }

  pub fn add_input(&mut self, id: ValueId) {
    self.inputs.push(id);
  }

  pub fn add_output(&mut self, ty: Option<Type>) {
    self.outputs.push(ty);
  }

  pub fn add_instance(&mut self, name: String, module: String) {
    self.instances.push(InstDef { name, module });
  }

  pub fn value_typ(&self, id: ValueId) -> Option<&Type> {
    self.values.get(id).and_then(|v| v.ty.as_ref())
  }

  pub fn ops(&self) -> impl Iterator<Item = &Op> {
    self.ops.iter()
  }

  pub fn ops_mut(&mut self) -> impl Iterator<Item = &mut Op> {
    self.ops.iter_mut()
  }

  pub fn replace_op(&mut self, target_op: &Op, new_ops: Vec<Op>) {
    let mut indices_to_replace = vec![];
    for (index, op) in self.ops.iter().enumerate() {
      if op == target_op {
        indices_to_replace.push(index);
      }
    }

    // Replace each matching operation
    for index in indices_to_replace.into_iter().rev() {
      let ops = new_ops.clone();
      self.ops.splice(index..=index, ops);
    }

    // Recursively replace sub-operations
    for op in &mut self.ops {
      op.replace_op(target_op, new_ops.clone());
    }
  }

  pub fn remove_unused_op(&mut self) {
    let mut indices_to_remove = vec![];

    for (index, op) in self.ops_mut().enumerate() {
      if let OpEnum::Assign(AssignOp { res, value }) = op.inner_mut() {
        if res == value {
          indices_to_remove.push(index);
        }
      } else {
        op.remove_unused_op();
      }
    }

    indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
    for &index in &indices_to_remove {
      self.ops.remove(index);
    }
  }
}

/// Circuit in `cmtir`.
/// (circuit <top_module_name> {<annotations>} (<functions>*) (<modules>*)
#[derive(Debug, Clone, SExpr)]
pub struct Circuit {
  #[pp(open)]
  #[pp(kw = "circuit")]
  /// circuit name = name of the top module.
  pub top_module_name: String,
  pub annotations: json::object::Object,
  #[pp(nl)]
  #[pp(list_ml)]
  pub functions: Vec<Function>,
  #[pp(list_ml)]
  #[pp(close)]
  pub modules: Vec<Module>,
}

impl Circuit {
  /// Create a circuit with the given top module name.
  pub fn with_top_module_name(name: String) -> Self {
    Circuit {
      top_module_name: name,
      annotations: json::object::Object::new(),
      modules: vec![],
      functions: vec![],
    }
  }
  /// Create a circuit with the given top module name and modules.
  pub fn new(name: String, modules: Vec<Module>) -> Self {
    Circuit {
      top_module_name: name,
      annotations: json::object::Object::new(),
      modules,
      functions: vec![],
    }
  }
  /// Get the top module of the circuit.
  pub fn top_module(&self) -> &Module {
    self
      .modules
      .iter()
      .find(|m| m.name == self.top_module_name)
      .unwrap()
  }

  /// Get all module names in the circuit.
  pub fn module_names(&self) -> impl Iterator<Item = &str> {
    self.modules.iter().map(|m| m.name.as_str())
  }

  /// Get all modules in the circuit.
  pub fn modules(&self) -> impl Iterator<Item = &Module> {
    self.modules.iter()
  }

  /// Get all modules in the circuit (mut).
  pub fn modules_mut(&mut self) -> impl Iterator<Item = &mut Module> {
    self.modules.iter_mut()
  }

  /// Get the index of the module by name.
  pub fn module_idx(&self, name: &str) -> usize {
    let module_names = self
      .modules
      .iter()
      .map(|m| m.name.clone())
      .collect::<Vec<String>>()
      .join(", ");
    self
      .modules
      .iter()
      .position(|m| m.name == name)
      .expect(format!("module {} not found in {}", name, module_names).as_str())
  }

  /// Get the module by name (mut).
  pub fn module_mut(&mut self, name: &str) -> &mut Module {
    let module_names = self
      .modules
      .iter()
      .map(|m| m.name.clone())
      .collect::<Vec<String>>()
      .join(", ");
    if let Some(module) = self.modules.iter_mut().find(|m| m.name == name) {
      module
    } else {
      log::error!("there are modules: {}", module_names);
      panic!("module {} not found", name);
    }
  }

  /// Get the module by name.
  pub fn module(&self, name: &str) -> Option<&Module> {
    self.modules.iter().find(|m| m.name == name)
  }

  /// Add a module to the circuit.
  pub fn add_module(&mut self, module: Module) -> String {
    let name = module.name().to_owned();
    if self.module(module.name()).is_none() {
      self.modules.push(module);
    }
    name
    // TODO: do we need to check if two modules of the same name are identical?
  }

  /// Get all function names in the circuit.
  pub fn function_names(&self) -> impl Iterator<Item = &str> {
    self.functions.iter().map(|f| f.name.as_str())
  }

  /// Add a function to the circuit.
  pub fn add_function(&mut self, function: Function) {
    self.functions.push(function);
  }

  /// Get all functions in the circuit.
  pub fn functions(&self) -> impl Iterator<Item = &Function> {
    self.functions.iter()
  }

  /// Get all functions in the circuit (mut).
  pub fn functions_mut(&mut self) -> impl Iterator<Item = &mut Function> {
    self.functions.iter_mut()
  }

  /// Get the index of the function by name.
  pub fn function_idx(&self, name: &str) -> usize {
    let function_names = self
      .functions
      .iter()
      .map(|m| m.name.clone())
      .collect::<Vec<String>>()
      .join(", ");
    self.functions.iter().position(|f| f.name == name).expect(
      format!("module {} not found in {}", name, function_names).as_str(),
    )
  }

  /// Get the output types of the rule in the module.
  pub fn rule_output_types(
    &self,
    module_name: &str,
    rule_name: &str,
  ) -> Result<Vec<Type>, String> {
    let module = self
      .module(module_name)
      .ok_or(format!("module {} not found", module_name))?;
    let rule = match module.rules().find(|r| r.name == rule_name) {
      Some(r) => r,
      None => return Ok(vec![]),
    };
    let outputs = rule.outputs();
    let output_types = outputs
      .iter()
      .map(|id| {
        module
          .value_typ(*id)
          .map(|t| t.clone())
          .ok_or(format!("value {} not found in module {}", id, module_name))
      })
      .collect::<Result<Vec<Type>, String>>()?;
    Ok(output_types)
  }
}

impl<T> From<T> for Circuit
where
  T: AsRef<str>,
{
  fn from(value: T) -> Self {
    let mut parser = Parser::new(value.as_ref());
    let circuit = parser.parse::<Circuit>().unwrap();
    circuit
  }
}
