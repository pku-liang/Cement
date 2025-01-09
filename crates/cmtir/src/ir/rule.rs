use std::collections::HashMap;

use super::*;

/// Rule signature in `cmtir`.
/// `always` rules have inputs and outputs
/// `method` rules have inputs, outputs, and side effect
#[derive(Debug, Clone, SExpr)]
pub enum RuleSignature {
  #[pp(surrounded)]
  Always {
    inputs: Vec<ValueId>,
    outputs: Vec<ValueId>,
  },
  Method {
    inputs: Vec<ValueId>,
    outputs: Vec<ValueId>,
    side_effect: Option<bool>,
  },
}

impl RuleSignature {
  /// Get all inputs of the rule.
  pub fn inputs(&self) -> impl Iterator<Item = ValueId> {
    match self {
      RuleSignature::Always { inputs, .. } => inputs.clone().into_iter(),
      RuleSignature::Method { inputs, .. } => inputs.clone().into_iter(),
    }
  }
  /// Get all inputs of the rule (mut).
  pub fn inputs_mut(&mut self) -> &mut Vec<ValueId> {
    match self {
      RuleSignature::Always { inputs, .. } => inputs,
      RuleSignature::Method { inputs, .. } => inputs,
    }
  }
  /// Get all outputs of the rule.
  pub fn outputs(&self) -> impl Iterator<Item = ValueId> {
    match self {
      RuleSignature::Always { outputs, .. } => outputs.clone().into_iter(),
      RuleSignature::Method { outputs, .. } => outputs.clone().into_iter(),
    }
  }
  /// Get all outputs of the rule (mut).
  pub fn outputs_mut(&mut self) -> &mut Vec<ValueId> {
    match self {
      RuleSignature::Always { outputs, .. } => outputs,
      RuleSignature::Method { outputs, .. } => outputs,
    }
  }

  pub fn is_method(&self) -> bool {
    matches!(self, RuleSignature::Method { .. })
  }
}

/// Rule timing in `cmtir`.
/// Currently, only single-cycle/FSM/pipeline rules are supported.
/// **multi-cycle rules are not supported yet.**
#[derive(Debug, Clone, SExpr)]
pub enum RuleTiming {
  #[pp(surrounded)]
  SingleCycle,
  MultiCycle {
    // Some(x) means latency-sensitive (x
    // cycles), None means latency-insensitive
    num_cycles: Option<u32>,
    intv: TimingIntv,
  },
  FSM,
  Pipeline,
}

impl RuleTiming {
  pub fn interval(&self) -> TimingIntv {
    match self {
      RuleTiming::SingleCycle => TimingIntv::none(),
      RuleTiming::MultiCycle {
        num_cycles: _,
        intv,
      } => intv.clone(),
      _ => unimplemented!(),
    }
  }
}

/// Rule in `cmtir`.
/// ((ext? <is_ext>) (private? <is_private>) rule "<name>" (<signature>)
/// (<timing>) <enable> <ready> (<guard_ops>) (<ops>)) <annotations>
#[derive(Debug, Clone, SExpr)]
pub struct Rule {
  #[pp(open)]
  #[pp(surrounded = "ext?")]
  pub is_ext: bool,
  #[pp(surrounded = "private?")]
  pub is_private: bool,
  #[pp(kw = "rule")]
  pub name: String,
  pub signature: RuleSignature,
  pub timing: RuleTiming,
  pub enable: Option<String>,
  pub ready: Option<String>,
  #[pp(list_ml)]
  pub guard_ops: Vec<Op>,
  #[pp(list_ml)]
  #[pp(close)]
  pub ops: Vec<Op>,
  pub annotations: json::object::Object,
}

impl Rule {
  /// Check if the rule is external.
  pub fn is_ext(&self) -> bool {
    self.is_ext
  }

  /// Check if the rule is private.
  pub fn is_private(&self) -> bool {
    self.is_private
  }

  /// Set the rule to private.
  pub fn set_private(&mut self, is_private: bool) {
    self.is_private = is_private;
  }

  /// Check if the rule is always.
  pub fn is_always(&self) -> bool {
    matches!(self.signature, RuleSignature::Always { .. })
  }

  /// Check if the rule is method.
  pub fn is_method(&self) -> bool {
    matches!(self.signature, RuleSignature::Method { .. })
  }

  /// Check if the rule has side effect.
  pub fn has_side_effect(&self) -> bool {
    matches!(self.signature, RuleSignature::Method { side_effect, .. } if side_effect.unwrap_or(false))
  }

  /// Set the rule to have side effect.
  pub fn set_side_effect(&mut self, se: bool) {
    if let RuleSignature::Method { side_effect, .. } = &mut self.signature {
      *side_effect = Some(se);
    }
  }

  /// Check if the rule is single-cycle.
  pub fn is_single_cycle(&self) -> bool {
    matches!(self.timing, RuleTiming::SingleCycle)
  }

  /// Get all guard operations of the rule.
  pub fn guard(&self) -> impl Iterator<Item = &Op> {
    self.guard_ops.iter()
  }

  /// Get all guard operations of the rule (mut).
  pub fn guard_mut(&mut self) -> impl Iterator<Item = &mut Op> {
    self.guard_ops.iter_mut()
  }

  /// Get all operations of the rule.
  pub fn ops(&self) -> impl Iterator<Item = &Op> {
    self.ops.iter()
  }

  /// Get all operations of the rule (mut).
  pub fn ops_mut(&mut self) -> impl Iterator<Item = &mut Op> {
    self.ops.iter_mut()
  }

  /// Get the name of the rule.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Create a new rule.
  pub fn new(name: String) -> Rule {
    Rule {
      is_ext: false,
      is_private: false,
      name,
      signature: RuleSignature::Always {
        inputs: vec![],
        outputs: vec![],
      },
      timing: RuleTiming::SingleCycle,
      guard_ops: vec![],
      ops: vec![],
      enable: None,
      ready: None,
      annotations: json::object::Object::new(),
    }
  }

  /// Create a new always rule.
  pub fn always(
    name: String,
    inputs: Vec<ValueId>,
    outputs: Vec<ValueId>,
    guard_ops: Vec<Op>,
    ops: Vec<Op>,
    timing: RuleTiming,
  ) -> Rule {
    Rule {
      is_ext: false,
      is_private: false,
      name,
      signature: RuleSignature::Always { inputs, outputs },
      guard_ops,
      timing,
      ops,
      enable: None,
      ready: None,
      annotations: json::object::Object::new(),
    }
  }

  /// Create a new method rule.
  pub fn method(
    name: String,
    inputs: Vec<ValueId>,
    outputs: Vec<ValueId>,
    side_effect: Option<bool>,
    guard_ops: Vec<Op>,
    ops: Vec<Op>,
    timing: RuleTiming,
  ) -> Rule {
    Rule {
      is_ext: false,
      is_private: false,
      name,
      signature: RuleSignature::Method {
        inputs,
        outputs,
        side_effect,
      },
      timing,
      guard_ops,
      ops,
      enable: None,
      ready: None,
      annotations: json::object::Object::new(),
    }
  }

  /// Create a new external rule.
  pub fn ext(
    name: String,
    inputs: Vec<ValueId>,
    outputs: Vec<ValueId>,
    enable: Option<String>,
    ready: Option<String>,
    side_effect: Option<bool>,
  ) -> Rule {
    Rule {
      is_ext: true,
      is_private: false,
      name,
      signature: RuleSignature::Method {
        inputs,
        outputs,
        side_effect,
      },
      guard_ops: vec![],
      timing: RuleTiming::SingleCycle,
      ops: vec![],
      enable,
      ready,
      annotations: json::object::Object::new(),
    }
  }

  /// Set the rule to private.
  pub fn to_be_private(self) -> Rule {
    Rule {
      is_private: true,
      ..self
    }
  }

  /// Get all spans of the rule from annotations.
  pub fn span(&self) -> impl Iterator<Item = MySpan> + '_ {
    self.annotations.iter().filter_map(|(k, v)| {
      if k.ends_with("span") {
        MySpan::from_json(v)
      } else {
        None
      }
    })
  }

  /// Get all inputs of the rule.
  pub fn inputs(&self) -> Vec<ValueId> {
    self.signature.inputs().collect()
  }

  /// Get all inputs of the rule (mut).
  pub fn inputs_mut(&mut self) -> &mut Vec<ValueId> {
    self.signature.inputs_mut()
  }

  /// Get all outputs of the rule.
  pub fn outputs(&self) -> Vec<ValueId> {
    self.signature.outputs().collect()
  }

  /// Get all outputs of the rule (mut).
  pub fn outputs_mut(&mut self) -> &mut Vec<ValueId> {
    self.signature.outputs_mut()
  }

  /// Get all return values of the rule.
  pub fn return_values(&self) -> Vec<ValueId> {
    // last op's outputs are the return values
    self
      .ops
      .last()
      .map(|op| op.outputs().collect())
      .unwrap_or_default()
  }

  /// Replace the guard operations of the rule.
  pub fn replace_guard_op(&mut self, index: usize, new_ops: Vec<Op>) {
    if index < self.guard_ops.len() {
      self.guard_ops.splice(index..=index, new_ops);
    } else {
      panic!("Index out of bounds for guard_ops");
    }
  }

  /// Replace the operations of the rule.
  pub fn replace_op(&mut self, index: usize, new_ops: Vec<Op>) {
    if index < self.ops.len() {
      self.ops.splice(index + 1..=index + 1, new_ops);
    } else {
      panic!("Index out of bounds for ops");
    }
  }

  /// Replace all operations of the rule with the given map.
  pub fn replace_all_op_with_map(
    &mut self,
    replacements: &HashMap<ir::ValueId, ir::ValueId>,
  ) {
    for op in self.guard_mut() {
      op.replace_value_with_map(replacements);
    }

    for op in self.ops_mut() {
      op.replace_value_with_map(replacements);
    }
  }

  /// Remove unused operations from the rule.
  pub fn remove_unused_op(&mut self) {
    let mut indices_to_remove = vec![];

    for (index, op) in self.guard_mut().enumerate() {
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
      self.guard_ops.remove(index);
    }

    indices_to_remove.clear();

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

/// Rule relation in `cmtir`.
/// `Method` relations specify constraints on the method rules, including
/// C(conflict), CF(conflict-free), SA(sequence-ahead), and SB(sequence-behind).
/// `Schedule` relations specify the order of the rules for execution.
#[derive(Debug, Clone, SExpr)]
pub enum RuleRel {
  #[pp(surrounded)]
  Method {
    rel: MethodRel,
    lhs: Vec<InstRule>,
    rhs: Vec<InstRule>,
  },
  Schedule(Vec<InstRule>),
}

impl RuleRel {
  /// Create a new method relation.
  pub fn method(
    rel: MethodRel,
    lhs: Vec<InstRule>,
    rhs: Vec<InstRule>,
  ) -> RuleRel {
    RuleRel::Method { rel, lhs, rhs }
  }

  /// Create a new schedule relation.
  pub fn schedule(inst_rules: Vec<InstRule>) -> RuleRel {
    RuleRel::Schedule(inst_rules)
  }
}
