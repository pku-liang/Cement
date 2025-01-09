//! Operations in `cmtir`.

use super::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use itertools::chain;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SExpr)]
pub enum Cmp {
  Eq, Neq, Lt, Gt, Leq, Geq,
}

/// "res cmp-enum a b"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct CmpOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(attr)]
  pub cmp: Cmp,
  #[opio(input)]
  pub a: ValueId,
  #[opio(input)]
  pub b: ValueId,
}

/// supported primitives in `cmtir`.
/// strongly consistent with [FIRRTL's primitives](https://github.com/chipsalliance/firrtl-spec/releases/tag/v4.0.0)
#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SExpr)]
pub enum Prim {
    Add, Sub, 
    Mul, Div, Rem,
    And, Or, Xor, 
    Andr, Orr, Xorr,
    Shl, Shr, DShl, DShr,
    Not, Neg,
    Cat,
    Pad,
    Cvt,
    AsUInt, AsSInt,
    Bits, Head, Tail,
}

/// "prim-enum (outputs) (inputs) (const attrs)"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct PrimOp {
  #[opio(attr)]
  pub prim: Prim,
  #[opio(output)]
  pub res: ValueId,
  #[opio(input)]
  pub inputs: Vec<ValueId>,
  #[opio(attr)]
  pub attrs: Vec<u32>,
}

/// "res <typed lit>"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct LitOp {
  #[opio(output)]
  pub res: ValueId,
  pub value: TypedLit,
}

/// "res <inst>.<rule> (args)"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct InvokeOp {
  #[opio(output)]
  pub res: Vec<ValueId>,
  #[opio(attr)]
  pub inst_rule: InstRule,
  #[opio(input)]
  pub args: Vec<ValueId>,
}

/// "res <function-name> (args) (instances)"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct CallOp {
  #[opio(output)]
  pub res: Vec<ValueId>,
  #[opio(attr)]
  pub function_name: String,
  #[opio(input)]
  pub args: Vec<ValueId>,
  #[opio(attr)]
  pub instances: Vec<String>,
}

/// "(ops)"
#[derive(Debug, Clone, SExpr)]
pub struct BodyOp {
  // #[pp(surrounded="or_not")]
  // pub or_not: bool,
  #[pp(list_ml)]
  pub ops: Vec<Op>,
}

impl BodyOp {
  /// Get all inputs of the body.
  pub fn inputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    let inputs = self.ops.iter().flat_map(|op| op.inputs());
    let outputs: HashSet<_> =
      self.ops.iter().flat_map(|op| op.outputs()).collect();
    inputs.filter(move |v| !outputs.contains(v))
  }
  /// Get all outputs of the body.
  pub fn outputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    self.ops.last().map(|op| op.outputs()).into_iter().flatten()
  }
}

impl OpIO for BodyOp {
  fn num_inputs(&self) -> usize {
    self.inputs().count()
  }

  fn input(&self, i: usize) -> ValueId {
    self.inputs().nth(i).unwrap()
  }

  fn input_mut(&mut self, _i: usize) -> &mut ValueId {
    unimplemented!("BodyOpEnum::input_mut")
  }

  fn num_outputs(&self) -> usize {
    self.outputs().count()
  }

  fn output(&self, i: usize) -> ValueId {
    self.outputs().nth(i).unwrap()
  }

  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    self.ops.last_mut().unwrap().output_mut(i)
  }
}

/// "(res) (cond) (then_body) (else_body)"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct IfOp {
  #[opio(output)]
  pub res: Vec<ValueId>,
  #[opio(input)]
  pub cond: ValueId,
  #[opio(wrap_input)]
  pub then_body: Op,
  #[opio(wrap_input)]
  pub else_body: Option<Op>,
}

#[derive(Debug, Clone, OpIO, SExpr)]
pub struct DelayOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(attr)]
  pub inst_rule: InstRule,
  #[opio(attr)]
  pub delay: u32,
}

#[derive(Debug, Clone, OpIO, SExpr)]
pub struct DynDelayOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(attr)]
  pub inst_rule: InstRule,
  #[opio(attr)]
  pub delay: u32,
}

#[derive(Debug, Clone, OpIO, SExpr)]
pub struct NopOp;

/// "res value"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct AssignOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(input)]
  pub value: ValueId,
}

/// either "name" or "index" for field access
#[derive(Debug, Clone, PartialEq, Eq, Hash, SExpr)]
pub enum Field {
  Name(String),
  Index(u32),
}

/// "values"
/// WARNING: violate SSA, since every returned value is defed multiple times;
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct ReturnOp {
  // #[opio(wrap_reverse)]
  // FIXME: violate SSA, since every returned value is defed multiple times;
  // currently, treat them specially in ValueContext
  #[opio(wrap_output)]
  pub values: Vec<ValueId>,
}

/// "res <field> <value>"
/// field access for bundle/vector
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct FieldOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(attr)]
  pub field: Field,
  #[opio(input)]
  pub value: ValueId,
}

/// "res (values) (fields)"
///  vector/bundle creation
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct AggregateOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(input)]
  pub values: Vec<ValueId>,
  #[opio(attr)]
  pub fields: Vec<Field>,
}

/// "(ops)"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct ParOp {
  #[pp(list_ml)]
  pub ops: Vec<Op>,
}

/// "<cond> <then_body> <else_body> <loose>"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct BranchOp {
  #[opio(wrap_input)]
  pub cond: Op,
  #[opio(wrap_input)]
  pub then_body: Op,
  #[opio(wrap_input)]
  pub else_body: Option<Op>,
  #[opio(attr)]
  pub loose: bool,
}

/// "<init> <init_cond> <cond> <update> <body> <loose>"
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct ForOp {
  // #[opio(input)]
  // pub var: ValueId,
  #[opio(wrap_input)]
  pub init: Option<Op>,
  #[opio(wrap_input)]
  pub init_cond: Option<Op>,
  #[opio(wrap_input)]
  pub cond: Op,
  #[opio(wrap_input)]
  pub update: Op,
  #[opio(wrap_input)]
  pub body: Op,
  #[opio(attr)]
  pub loose: bool,
}

/// items for printing during simulation
/// either "string" or "value"
#[derive(Debug, Clone, Hash, SExpr)]
pub enum PrintItem {
  String(String),
  Value(ValueId),
}

/// "items"
#[derive(Debug, Clone, Hash, OpIO, SExpr)]
pub struct SimPrintOp {
  pub items: Vec<PrintItem>,
}

/// mark simulation exit
#[derive(Debug, Clone, Hash, OpIO, SExpr)]
pub struct SimExitOp {}

/// convert integer to hardware signal during simulation
#[derive(Debug, Clone, Hash, OpIO, SExpr)]
pub struct SimFromInt {
  #[opio(output)]
  pub res: ValueId,
  #[opio(input)]
  pub a: ValueId,
  // pub ty: Type,
}

/// Enum of all operations in `cmtir`.
/// (type op)
#[derive(Debug, Clone, OpIO, SExpr)]
pub enum OpEnum {
  #[pp(surrounded)]
  Nop(NopOp),
  Assign(AssignOp),
  Lit(LitOp),
  Cmp(CmpOp),
  Prim(PrimOp),
  Invoke(InvokeOp),
  Call(CallOp),
  Field(FieldOp),
  Aggregate(AggregateOp),
  Timed(TimingIntv, Box<Op>),
  Block(Box<BodyOp>),
  If(Box<IfOp>),
  Return(ReturnOp),
  Delay(DelayOp),
  DynDelay(DynDelayOp),
  Step(Box<BodyOp>),
  Seq(Box<BodyOp>),
  Par(Box<ParOp>),
  Branch(Box<BranchOp>),
  For(Box<ForOp>),
  SimPrint(SimPrintOp),
  SimExit(SimExitOp),
  SimFromInt(SimFromInt),
}

/// An operation in `cmtir`.
/// composed of an operation enum and its annotations.
#[derive(Debug, Clone, SExpr)]
pub struct Op {
  pub inner: OpEnum,
  pub annotations: json::object::Object,
}

impl OpIO for Op {
  fn num_inputs(&self) -> usize {
    self.inner.num_inputs()
  }

  fn input(&self, i: usize) -> ValueId {
    self.inner.input(i)
  }

  fn input_mut(&mut self, i: usize) -> &mut ValueId {
    self.inner.input_mut(i)
  }

  fn num_outputs(&self) -> usize {
    self.inner.num_outputs()
  }

  fn output(&self, i: usize) -> ValueId {
    self.inner.output(i)
  }

  fn output_mut(&mut self, i: usize) -> &mut ValueId {
    self.inner.output_mut(i)
  }
}

impl Into<Op> for OpEnum {
  fn into(self) -> Op {
    Op {
      inner: self,
      annotations: json::object::Object::new(),
    }
  }
}

impl Into<Op> for Vec<Op> {
  fn into(self) -> Op {
    Op {
      inner: OpEnum::Block(Box::new(BodyOp { ops: self })),
      annotations: json::object::Object::new(),
    }
  }
}

impl PartialEq for CallOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.function_name == other.function_name
      && self.args == other.args
      && self.instances == other.instances
  }
}

impl Op {
  /// Get the span of the operation from annotations.
  pub fn span(&self) -> Option<MySpan> {
    self.annotations.iter().find_map(|(k, v)| {
      if k.ends_with("span") {
        MySpan::from_json(v)
      } else {
        None
      }
    })
  }
  /// Set the span of the operation.  
  pub fn with_span(mut self, span: Option<MySpan>, prefix: &str) -> Op {
    if let Some(span) = span {
      self
        .annotations
        .insert(&format!("{}_span", prefix), span.into());
    }
    self
  }
  /// Get the inner operation enum.
  pub fn inner_mut(&mut self) -> &mut OpEnum {
    &mut self.inner
  }
  /// Get the inner operation enum.
  pub fn inner(&self) -> &OpEnum {
    &self.inner
  }
  /// Get the kind of the operation.
  pub fn kind_str(&self) -> String {
    match &self.inner {
      OpEnum::Nop(_) => "Nop".to_string(),
      OpEnum::Assign(_) => "Assign".to_string(),
      OpEnum::Lit(_) => "Lit".to_string(),
      OpEnum::Cmp(_) => "Cmp".to_string(),
      OpEnum::Prim(_) => "Prim".to_string(),
      OpEnum::Invoke(_) => "Invoke".to_string(),
      OpEnum::Call(_) => "Call".to_string(),
      OpEnum::Field(_) => "Field".to_string(),
      OpEnum::Aggregate(_) => "Aggregate".to_string(),
      OpEnum::Timed(_, _) => "Timed".to_string(),
      OpEnum::Block(_) => "Block".to_string(),
      OpEnum::If(_) => "If".to_string(),
      OpEnum::Return(_) => "Return".to_string(),
      OpEnum::Delay(_) => "Delay".to_string(),
      OpEnum::DynDelay(_) => "DynDelay".to_string(),
      OpEnum::Step(_) => "Step".to_string(),
      OpEnum::Seq(_) => "Seq".to_string(),
      OpEnum::Par(_) => "Par".to_string(),
      OpEnum::Branch(_) => "Branch".to_string(),
      OpEnum::For(_) => "For".to_string(),
      OpEnum::SimPrint(_) => "SimPrint".to_string(),
      OpEnum::SimExit(_) => "SimExit".to_string(),
      OpEnum::SimFromInt(_) => "SimFromInt".to_string(),
    }
  }
  /// Check if the operation is a return operation.
  pub fn is_return(&self) -> bool {
    matches!(&self.inner, OpEnum::Return(_))
  }
  /// Check if the operation is an invoke operation.
  pub fn is_invoke(&self) -> bool {
    matches!(&self.inner, OpEnum::Invoke(_))
  }
  /// Get the invoke operation reference.
  pub fn as_invoke_op_ref(&self) -> Option<&InvokeOp> {
    match &self.inner {
      OpEnum::Invoke(invoke) => Some(invoke),
      _ => None,
    }
  }
  /// Get the invoke operation.
  pub fn as_invoke_op(self) -> Option<InvokeOp> {
    match self.inner {
      OpEnum::Invoke(invoke) => Some(invoke),
      _ => None,
    }
  }
  /// Get all sub-operations of the operation.
  pub fn sub_ops(&self) -> impl Iterator<Item = &Op> {
    let ops: Vec<&Op> = match &self.inner {
      OpEnum::Block(body) => body.ops.iter().collect(),
      OpEnum::If(if_op) => {
        chain!(vec![&if_op.then_body], if_op.else_body.as_ref().into_iter())
          .collect()
      }
      OpEnum::Timed(_, op) => vec![op.as_ref()],
      _ => vec![],
    };
    ops.into_iter()
  }
  /// Get all sub-operations of the operation (mut).
  pub fn sub_ops_mut(&mut self) -> impl Iterator<Item = &mut Op> {
    let ops: Vec<&mut Op> = match &mut self.inner {
      OpEnum::Block(body) => body.ops.iter_mut().collect(),
      OpEnum::If(if_op) => chain!(
        vec![&mut if_op.then_body],
        if_op.else_body.as_mut().into_iter()
      )
      .collect(),
      OpEnum::Timed(_, op) => vec![op],
      _ => vec![],
    };
    ops.into_iter()
  }

  /// Create a timed operation.
  pub fn timed(interval: TimingIntv, op: Op) -> Op {
    Op {
      inner: OpEnum::Timed(interval, Box::new(op)),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a delay operation.
  pub fn delay(inst_rule: InstRule, delay: u32, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Delay(DelayOp {
        inst_rule,
        delay,
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a dynamic delay operation.
  pub fn dyn_delay(inst_rule: InstRule, delay: u32, res: ValueId) -> Op {
    Op {
      inner: OpEnum::DynDelay(DynDelayOp {
        inst_rule,
        delay,
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a return operation.
  pub fn none() -> Op {
    Op {
      inner: OpEnum::Return(ReturnOp { values: vec![] }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a compare operation.
  pub fn cmp(cmp: Cmp, a: ValueId, b: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Cmp(CmpOp { cmp, a, b, res }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a primitive operation.
  pub fn prim(
    prim: Prim,
    res: ValueId,
    inputs: Vec<ValueId>,
    attrs: Vec<u32>,
  ) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim,
        res,
        inputs,
        attrs,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a field operation.
  pub fn field(value: ValueId, field: Field, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Field(FieldOp { value, field, res }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a bundle operation.
  pub fn create_struct(
    values: Vec<ValueId>,
    fields: Vec<String>,
    res: ValueId,
  ) -> Op {
    Op {
      inner: OpEnum::Aggregate(AggregateOp {
        values,
        fields: fields.into_iter().map(|f| Field::Name(f)).collect(),
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a vector operation.
  pub fn create_vector(values: Vec<ValueId>, res: ValueId) -> Op {
    let num = values.len();
    Op {
      inner: OpEnum::Aggregate(AggregateOp {
        values: values,
        fields: (0..num).map(|i| Field::Index(i as u32)).collect(),
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create an add operation.
  pub fn add(a: ValueId, b: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim: Prim::Add,
        res,
        inputs: vec![a, b],
        attrs: vec![],
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a binary operation.
  pub fn binary_op(op: Prim, a: ValueId, b: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim: op,
        res,
        inputs: vec![a, b],
        attrs: vec![],
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a shift operation.
  pub fn shift_op(op: Prim, a: ValueId, n: u32, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim: op,
        res,
        inputs: vec![a],
        attrs: vec![n],
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a unary operation.
  pub fn unary_op(op: Prim, a: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim: op,
        res,
        inputs: vec![a],
        attrs: vec![],
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a not operation.
  pub fn not(a: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Prim(PrimOp {
        prim: Prim::Not,
        res,
        inputs: vec![a],
        attrs: vec![],
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a literal operation.
  pub fn lit(int: TypedLit, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Lit(LitOp { value: int, res }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a literal operation with integer.
  pub fn lit_int(int: u32, width: u32, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Lit(LitOp {
        value: TypedLit::new(int.into(), Type::UInt(width)),
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a block operation.
  pub fn block(ops: Vec<Op>) -> Op {
    Op {
      inner: OpEnum::Block(Box::new(BodyOp { ops })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a block operation with optional.
  pub fn block_or_not(ops: Vec<Op>) -> Op {
    Op {
      inner: OpEnum::Block(Box::new(BodyOp { ops })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a block operation with optional.
  pub fn op_or_not(op: Op) -> Op {
    Op {
      inner: OpEnum::Block(Box::new(BodyOp { ops: vec![op] })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a return operation.
  pub fn ret(values: Vec<ValueId>) -> Op {
    Op {
      inner: OpEnum::Return(ReturnOp { values }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create an if operation.
  pub fn if_(
    cond: ValueId,
    res: Vec<ValueId>,
    then_body: Op,
    else_body: Option<Op>,
  ) -> Op {
    Op {
      inner: OpEnum::If(Box::new(IfOp {
        cond,
        res,
        then_body,
        else_body,
      })),
      annotations: json::object::Object::new(),
    }
  }
  /// Create an invoke operation.
  pub fn invoke(
    inst_rule: InstRule,
    args: Vec<ValueId>,
    res: Vec<ValueId>,
  ) -> Op {
    Op {
      inner: OpEnum::Invoke(InvokeOp {
        inst_rule,
        args,
        res,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a call operation.
  pub fn call(
    res: Vec<ValueId>,
    function_name: String,
    args: Vec<ValueId>,
    instances: Vec<String>,
  ) -> Op {
    Op {
      inner: OpEnum::Call(CallOp {
        res,
        function_name,
        args,
        instances,
      }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create an assign operation.
  pub fn assign(value: ValueId, res: ValueId) -> Op {
    Op {
      inner: OpEnum::Assign(AssignOp { value, res }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a simulation print operation.
  pub fn sim_print(items: Vec<PrintItem>) -> ir::Op {
    ir::Op {
      inner: OpEnum::SimPrint(SimPrintOp { items }),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a simulation exit operation.
  pub fn sim_exit() -> ir::Op {
    ir::Op {
      inner: OpEnum::SimExit(SimExitOp {}),
      annotations: json::object::Object::new(),
    }
  }
  /// Create a simulation from integer operation. 
  pub fn sim_from_int(res: ValueId, a: ValueId) -> ir::Op {
    ir::Op {
      inner: OpEnum::SimFromInt(SimFromInt { res, a }),
      annotations: json::object::Object::new(),
    }
  }

  /// Replace the target operation with new operations.
  pub fn replace_op(&mut self, target_op: &Op, new_ops: Vec<Op>) {
    match &mut self.inner {
      // Handle block operation
      OpEnum::Block(body_op) => {
        // Collect the indices of the matching operations
        let mut indices_to_replace = vec![];
        for (index, op) in body_op.ops.iter().enumerate() {
          if op == target_op {
            indices_to_replace.push(index);
          }
        }

        // Replace each matching operation
        for index in indices_to_replace.into_iter().rev() {
          let ops = new_ops.clone();
          body_op.ops.splice(index..=index, ops);
        }

        // Recursively replace sub-operations
        for op in &mut body_op.ops {
          op.replace_op(target_op, new_ops.clone());
        }
      }
      // Handle conditional operations
      OpEnum::If(if_op) => {
        if let Some(else_body) = &mut if_op.else_body {
          else_body.replace_op(target_op, new_ops.clone());
        }
        if_op.then_body.replace_op(target_op, new_ops.clone());
      }
      // Handle timed operations
      OpEnum::Timed(_, sub_op) => {
        sub_op.replace_op(target_op, new_ops.clone());
      }
      // Handle other cases recursively if needed
      _ => {}
    }
  }

  /// Replace the instance name in the operation.
  pub fn replace_instance(
    &mut self,
    old_instance: String,
    new_instance: String,
  ) {
    match &mut self.inner {
      OpEnum::Invoke(invoke_op) => {
        if let Some(inst_name) = invoke_op.inst_rule.path.get(0) {
          if *inst_name == old_instance {
            invoke_op.inst_rule.path[0] = new_instance;
          }
        }
      }
      _ => {}
    }
  }

  /// Replace the value with the map.
  pub fn replace_value_with_map(
    &mut self,
    replacements: &HashMap<ir::ValueId, ir::ValueId>,
  ) {
    match &mut self.inner {
      OpEnum::Cmp(cmp_op) => {
        if let Some(&new_id) = replacements.get(&cmp_op.a) {
          cmp_op.a = new_id;
        }
        if let Some(&new_id) = replacements.get(&cmp_op.b) {
          cmp_op.b = new_id;
        }
        if let Some(&new_id) = replacements.get(&cmp_op.res) {
          cmp_op.res = new_id;
        }
      }
      OpEnum::Prim(prim_op) => {
        if let Some(&new_id) = replacements.get(&prim_op.res) {
          prim_op.res = new_id;
        }
        for input in &mut prim_op.inputs {
          if let Some(&new_id) = replacements.get(input) {
            *input = new_id;
          }
        }
      }
      OpEnum::Lit(lit_op) => {
        if let Some(&new_id) = replacements.get(&lit_op.res) {
          lit_op.res = new_id;
        }
      }
      OpEnum::Invoke(invoke_op) => {
        for arg in &mut invoke_op.args {
          if let Some(&new_id) = replacements.get(arg) {
            *arg = new_id;
          }
        }
        for res in &mut invoke_op.res {
          if let Some(&new_id) = replacements.get(res) {
            *res = new_id;
          }
        }
      }
      OpEnum::Call(call_op) => {
        for arg in &mut call_op.args {
          if let Some(&new_id) = replacements.get(arg) {
            *arg = new_id;
          }
        }
      }
      OpEnum::Assign(assign_op) => {
        if let Some(&new_id) = replacements.get(&assign_op.value) {
          assign_op.value = new_id;
        }
        if let Some(&new_id) = replacements.get(&assign_op.res) {
          assign_op.res = new_id;
        }
      }
      OpEnum::Delay(delay_op) => {
        if let Some(&new_id) = replacements.get(&delay_op.res) {
          delay_op.res = new_id;
        }
      }
      OpEnum::If(if_op) => {
        if let Some(&new_id) = replacements.get(&if_op.cond) {
          if_op.cond = new_id;
        }
        for res in &mut if_op.res {
          if let Some(&new_id) = replacements.get(res) {
            *res = new_id;
          }
        }
        if_op.then_body.replace_value_with_map(replacements);
        if let Some(else_body) = &mut if_op.else_body {
          else_body.replace_value_with_map(replacements);
        }
      }
      OpEnum::Block(body_op) => {
        for op in &mut body_op.ops {
          op.replace_value_with_map(replacements);
        }
      }
      OpEnum::Timed(_, op) => {
        op.replace_value_with_map(replacements);
      }
      ir::OpEnum::Return(ReturnOp { values }) => {
        for arg in values {
          if let Some(&new_id) = replacements.get(arg) {
            *arg = new_id;
          }
        }
      }
      _ => {}
    }
  }

  /// Remove the unused operation.
  pub fn remove_unused_op(&mut self) {
    match &mut self.inner {
      // Handle block operation
      OpEnum::Block(body_op) => {
        // Collect the indices of the matching operations
        let mut indices_to_remove = vec![];
        for (index, op) in body_op.ops.iter().enumerate() {
          if let OpEnum::Assign(AssignOp { res, value }) = &op.inner {
            if res == value {
              indices_to_remove.push(index);
            }
          }
        }

        indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for &index in &indices_to_remove {
          if index < body_op.ops.len() {
            body_op.ops.remove(index);
          }
        }
      }
      // Handle conditional operations
      OpEnum::If(if_op) => {
        if let Some(else_body) = &mut if_op.else_body {
          else_body.remove_unused_op();
        }
        if_op.then_body.remove_unused_op();
      }
      // Handle timed operations
      OpEnum::Timed(_, sub_op) => {
        sub_op.remove_unused_op();
      }
      // Handle other cases recursively if needed
      _ => {}
    }
  }
}

impl Hash for Op {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match &self.inner {
      OpEnum::Nop(_) => {
        "Nop".hash(state);
      }
      OpEnum::Assign(op) => {
        "Assign".hash(state);
        op.hash(state);
      }
      OpEnum::Lit(op) => {
        "Lit".hash(state);
        op.hash(state);
      }
      OpEnum::Cmp(op) => {
        "Cmp".hash(state);
        op.hash(state);
      }
      OpEnum::Prim(op) => {
        "Prim".hash(state);
        op.hash(state);
      }
      OpEnum::Invoke(op) => {
        "Invoke".hash(state);
        op.hash(state);
      }
      OpEnum::Field(op) => {
        "Field".hash(state);
        op.hash(state);
      }
      OpEnum::Aggregate(op) => {
        "Aggregate".hash(state);
        op.hash(state);
      }
      OpEnum::Timed(interval, op) => {
        "Timed".hash(state);
        interval.start.hash(state);
        interval.end.hash(state);
        op.hash(state);
      }
      OpEnum::Block(body) => {
        "Block".hash(state);
        body.hash(state);
      }
      OpEnum::If(if_op) => {
        "If".hash(state);
        if_op.hash(state);
      }
      OpEnum::Return(op) => {
        "Return".hash(state);
        op.hash(state);
      }
      OpEnum::Delay(op) => {
        "Delay".hash(state);
        op.hash(state);
      }
      OpEnum::DynDelay(op) => {
        "DynDelay".hash(state);
        op.hash(state);
      }
      OpEnum::Call(op) => {
        "Call".hash(state);
        op.hash(state);
      }
      OpEnum::Step(op) => {
        "Step".hash(state);
        op.hash(state);
      }
      OpEnum::Seq(op) => {
        "Seq".hash(state);
        op.hash(state);
      }
      OpEnum::Par(op) => {
        "Par".hash(state);
        op.hash(state);
      }
      OpEnum::Branch(op) => {
        "Branch".hash(state);
        op.hash(state);
      }
      OpEnum::For(op) => {
        "For".hash(state);
        op.hash(state);
      }
      OpEnum::SimPrint(op) => {
        "SimPrint".hash(state);
        op.hash(state);
      }
      OpEnum::SimExit(_) => {
        "SimExit".hash(state);
      }
      OpEnum::SimFromInt(op) => {
        "SimFromInt".hash(state);
        op.hash(state);
      }
    }
  }
}

impl std::hash::Hash for CmpOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.cmp.hash(state);
    self.a.hash(state);
    self.b.hash(state);
  }
}

impl std::hash::Hash for PrimOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.prim.hash(state);
    self.res.hash(state);
    for input in &self.inputs {
      input.hash(state);
    }
    for attr in &self.attrs {
      attr.hash(state);
    }
  }
}

impl std::hash::Hash for LitOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.value.hash(state);
  }
}

impl std::hash::Hash for InvokeOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for r in &self.res {
      r.hash(state);
    }
    self.inst_rule.hash(state);
    for arg in &self.args {
      arg.hash(state);
    }
  }
}


impl std::hash::Hash for BodyOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for op in &self.ops {
      op.hash(state);
    }
  }
}

impl std::hash::Hash for IfOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for r in &self.res {
      r.hash(state);
    }
    self.cond.hash(state);
    self.then_body.hash(state);
    if let Some(else_body) = &self.else_body {
      else_body.hash(state);
    }
  }
}

impl std::hash::Hash for DelayOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.inst_rule.hash(state);
    self.delay.hash(state);
  }
}

impl std::hash::Hash for DynDelayOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.inst_rule.hash(state);
    self.delay.hash(state);
  }
}

impl std::hash::Hash for AssignOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.value.hash(state);
  }
}

impl std::hash::Hash for FieldOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.value.hash(state);
  }
}

impl std::hash::Hash for AggregateOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    for value in &self.values {
      value.hash(state);
    }
    for field in &self.fields {
      field.hash(state);
    }
  }
}

impl std::hash::Hash for TypedLit {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.lit.hash(state);
    self.ty.hash(state);
  }
}

impl std::hash::Hash for ReturnOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for value in &self.values {
      value.hash(state);
    }
  }
}

impl std::hash::Hash for CallOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.res.hash(state);
    self.function_name.hash(state);
    for arg in &self.args {
      arg.hash(state);
    }
  }
}

impl std::hash::Hash for ParOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.ops.hash(state);
  }
}

impl std::hash::Hash for BranchOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.cond.hash(state);
    self.then_body.hash(state);
    if let Some(else_body) = &self.else_body {
      else_body.hash(state);
    }
  }
}

impl std::hash::Hash for ForOp {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.body.hash(state);
  }
}

impl PartialEq for Op {
  fn eq(&self, other: &Self) -> bool {
    match (&self.inner, &other.inner) {
      (OpEnum::Nop(lhs), OpEnum::Nop(rhs)) => lhs == rhs,
      (OpEnum::Assign(lhs), OpEnum::Assign(rhs)) => lhs == rhs,
      (OpEnum::Lit(lhs), OpEnum::Lit(rhs)) => lhs == rhs,
      (OpEnum::Cmp(lhs), OpEnum::Cmp(rhs)) => lhs == rhs,
      (OpEnum::Prim(lhs), OpEnum::Prim(rhs)) => lhs == rhs,
      (OpEnum::Invoke(lhs), OpEnum::Invoke(rhs)) => lhs == rhs,
      (OpEnum::Field(lhs), OpEnum::Field(rhs)) => lhs == rhs,
      (OpEnum::Timed(lhs, lhs_op), OpEnum::Timed(rhs, rhs_op)) => {
        lhs.start == rhs.start && lhs.end == rhs.end && lhs_op == rhs_op
      }
      (OpEnum::Block(lhs), OpEnum::Block(rhs)) => lhs == rhs,
      (OpEnum::If(lhs), OpEnum::If(rhs)) => lhs == rhs,
      (OpEnum::Return(lhs), OpEnum::Return(rhs)) => lhs == rhs,
      (OpEnum::Delay(lhs), OpEnum::Delay(rhs)) => lhs == rhs,
      (OpEnum::DynDelay(lhs), OpEnum::DynDelay(rhs)) => lhs == rhs,
      (OpEnum::Call(lhs), OpEnum::Call(rhs)) => lhs == rhs,
      _ => false,
    }
  }
}

impl PartialEq for NopOp {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

impl PartialEq for CmpOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.cmp == other.cmp
      && self.a == other.a
      && self.b == other.b
  }
}

impl PartialEq for PrimOp {
  fn eq(&self, other: &Self) -> bool {
    self.prim == other.prim
      && self.res == other.res
      && self.inputs == other.inputs
      && self.attrs == other.attrs
  }
}

impl PartialEq for InvokeOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.inst_rule == other.inst_rule
      && self.args == other.args
  }
}

impl PartialEq for BodyOp {
  fn eq(&self, other: &Self) -> bool {
    self.ops == other.ops
  }
}

impl PartialEq for IfOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.cond == other.cond
      && self.then_body == other.then_body
      && self.else_body == other.else_body
  }
}

impl PartialEq for DelayOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.inst_rule == other.inst_rule
      && self.delay == other.delay
  }
}

impl PartialEq for DynDelayOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.inst_rule == other.inst_rule
      && self.delay == other.delay
  }
}

impl PartialEq for AssignOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res && self.value == other.value
  }
}

impl PartialEq for FieldOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res && self.value == other.value
  }
}

impl PartialEq for LitOp {
  fn eq(&self, other: &Self) -> bool {
    self.res == other.res
      && self.value.lit == other.value.lit
      && self.value.ty == other.value.ty
  }
}

impl PartialEq for ReturnOp {
  fn eq(&self, other: &Self) -> bool {
    self.values == other.values
  }
}

impl Into<fir::PrimOp> for ir::Cmp {
  fn into(self) -> fir::PrimOp {
    match self {
      ir::Cmp::Eq => fir::PrimOp::Eq,
      ir::Cmp::Neq => fir::PrimOp::Neq,
      ir::Cmp::Lt => fir::PrimOp::Lt,
      ir::Cmp::Gt => fir::PrimOp::Gt,
      ir::Cmp::Leq => fir::PrimOp::Leq,
      ir::Cmp::Geq => fir::PrimOp::Geq,
    }
  }
}

impl Into<fir::PrimOp> for ir::Prim {
  fn into(self) -> fir::PrimOp {
    match self {
      ir::Prim::Add => fir::PrimOp::Add,
      ir::Prim::Sub => fir::PrimOp::Sub,
      ir::Prim::And => fir::PrimOp::And,
      ir::Prim::Or => fir::PrimOp::Or,
      ir::Prim::Xor => fir::PrimOp::Xor,
      ir::Prim::Not => fir::PrimOp::Not,
      ir::Prim::Shl => fir::PrimOp::Shl,
      ir::Prim::Mul => fir::PrimOp::Mul,
      ir::Prim::Div => fir::PrimOp::Div,
      ir::Prim::Rem => fir::PrimOp::Rem,
      ir::Prim::Shr => fir::PrimOp::Shr,
      ir::Prim::Andr => fir::PrimOp::Andr,
      ir::Prim::Orr => fir::PrimOp::Orr,
      ir::Prim::Xorr => fir::PrimOp::Xorr,
      ir::Prim::Neg => fir::PrimOp::Neg,
      ir::Prim::Cat => fir::PrimOp::Cat,
      ir::Prim::Pad => fir::PrimOp::Pad,
      ir::Prim::Cvt => fir::PrimOp::Cvt,
      ir::Prim::AsUInt => fir::PrimOp::AsUInt,
      ir::Prim::AsSInt => fir::PrimOp::AsSInt,
      ir::Prim::Bits => fir::PrimOp::Bits,
      ir::Prim::Head => fir::PrimOp::Head,
      ir::Prim::Tail => fir::PrimOp::Tail,
      ir::Prim::DShl => fir::PrimOp::Dshl,
      ir::Prim::DShr => fir::PrimOp::Dshr,
    }
  }
}

impl std::cmp::Eq for ir::Op {}
