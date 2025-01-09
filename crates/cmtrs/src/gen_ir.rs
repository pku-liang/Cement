use std::collections::HashSet;

use cmtir::{json, CmtError, ExtModuleBody, InstDef, MySpan, NativeModuleBody, OpEnum};
pub use cmtir::{
  Cmp, InstRule, MethodRel, Module, ModuleBody, NopOp, Op, Prim, PrintItem, Rule, RuleRel, RuleSignature, RuleTiming,
  SimExitOp, SlotMap, TimingIntv, TypedLit, Value, ValueId,
};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::*;

macro_rules! flatten_one {
  ($a: expr, $slotmap: expr, $results: expr) => {{
    let _ret = $a.flatten(1, $slotmap, $results);
    assert_eq!(_ret.len(), 1);
    _ret.into_iter().next().unwrap()
  }};
  ($a: expr, $slotmap: expr, $span: expr, $results: expr) => {{
    let _ret = $a.flatten(1, $slotmap, $span, $results);
    assert_eq!(_ret.len(), 1);
    _ret.into_iter().next().unwrap()
  }};
}
macro_rules! flatten_op {
  ($a: expr, $slotmap: expr, $expect_ret: expr) => {{
    let mut ops = Vec::new();
    let _ret = $a.flatten($expect_ret, $slotmap, &mut ops);
    assert!(_ret.len() == $expect_ret || _ret.len() == 0);
    assert_eq!(ops.len(), 1);
    ops.into_iter().next().unwrap()
  }};
}
macro_rules! flatten_block {
  ($a: expr, $slotmap: expr, $expect_ret: expr) => {{
    let block = BodyOpMold { ops: $a };
    let mut ops = Vec::new();
    let _ret = block.flatten($expect_ret, $slotmap, None, &mut ops);
    assert!(_ret.len() == $expect_ret || _ret.len() == 0);
    assert_eq!(ops.len(), 1);
    ops.into_iter().next().unwrap()
  }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(usize);

trait OpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId>;
}

trait StatementMold {
  fn statement(&mut self, ast: AST);
  fn sep(&mut self);
  fn end(&mut self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ValueMold {
  pub id: ValueId,
}
impl OpMold for ValueMold {
  fn flatten(
    self, expect_ret: usize, _slotmap: &mut SlotMap<ValueId, Value>, _span: Option<MySpan>, _results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    vec![self.id]
  }
}

#[derive(Debug, Clone)]
pub struct MultiValueMold {
  pub ids: Vec<ValueId>,
}
impl OpMold for MultiValueMold {
  fn flatten(
    self, expect_ret: usize, _slotmap: &mut SlotMap<ValueId, Value>, _span: Option<MySpan>, _results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret == 0 || expect_ret == self.ids.len());
    self.ids
  }
}

#[derive(Debug, Clone)]
pub struct CmpOpMold {
  pub cmp: Cmp,
  pub a: Option<AST>,
  pub b: Option<AST>,
}
impl OpMold for CmpOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    let a = flatten_one!(self.a.unwrap(), slotmap, results);
    let b = flatten_one!(self.b.unwrap(), slotmap, results);
    let res = slotmap.insert(Value::new_wo_ty(None));
    results.push(Op::cmp(self.cmp, a, b, res).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub struct PrimOpMold {
  pub prim: Prim,
  pub inputs: Vec<AST>,
  pub attrs: Vec<u32>,
}
impl OpMold for PrimOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    let inputs = self.inputs.into_iter().map(|x| flatten_one!(x, slotmap, results)).collect();
    let res = slotmap.insert(Value::new_wo_ty(None));
    results.push(Op::prim(self.prim, res, inputs, self.attrs).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub struct LitOpMold {
  pub value: TypedLit,
}
impl OpMold for LitOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    let res = slotmap.insert(Value::new_wo_ty(None));
    results.push(Op::lit(self.value, res).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub struct BodyOpMold {
  pub ops: Vec<AST>,
}
impl OpMold for BodyOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    let mut ops = Vec::new();
    let n = self.ops.len();
    let mut res = None;
    for (i, op) in self.ops.into_iter().enumerate() {
      let rst = op.flatten(if i != n - 1 { 0 } else { expect_ret }, slotmap, &mut ops);
      if i != n - 1 {
        assert!(rst.is_empty(), "Expect no return value at the middle of a block");
      } else {
        res = Some(rst);
      }
    }
    results.push(Op::block(ops).with_span(span, "cmtrs"));

    if let Some(r) = res {
      assert!(expect_ret == r.len() || expect_ret == 0);
      r
    } else {
      Vec::new()
    }
  }
}
impl StatementMold for BodyOpMold {
  fn statement(&mut self, ast: AST) { self.ops.push(ast); }

  fn sep(&mut self) {}

  fn end(&mut self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct InvokeOpMold {
  // pub res: Vec<ValueId>,
  pub inst_rule: InstRule,
  pub args: Vec<AST>,
  pub num_res: usize,
}

impl OpMold for InvokeOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret == 0 || expect_ret == self.num_res);
    let args = self.args.into_iter().map(|x| flatten_one!(x, slotmap, results)).collect();
    let res: Vec<ValueId> = (0..self.num_res).map(|_| slotmap.insert(Value::new_wo_ty(None))).collect();
    results.push(Op::invoke(self.inst_rule, args, res.clone()).with_span(span, "cmtrs"));
    res
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum IfGenState {
  Cond,
  Then,
  Else,
}

#[derive(Debug, Clone)]
pub struct IfOpMold {
  pub cond: Vec<AST>,
  pub then_body: Vec<AST>,
  pub else_body: Vec<AST>,
  state: IfGenState,
}

impl OpMold for IfOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    let cond = {
      let block = BodyOpMold { ops: self.cond };
      flatten_one!(block, slotmap, None, results)
    };
    let then_body = flatten_block!(self.then_body, slotmap, expect_ret);

    let else_body =
      if !self.else_body.is_empty() { Some(flatten_block!(self.else_body, slotmap, expect_ret)) } else { None };
    let res: Vec<ValueId> = (0..expect_ret).map(|_| slotmap.insert(Value::new_wo_ty(None))).collect();
    results.push(Op::if_(cond, res.clone(), then_body, else_body).with_span(span, "cmtrs"));
    res
  }
}
impl StatementMold for IfOpMold {
  fn statement(&mut self, ast: AST) {
    match self.state {
      IfGenState::Cond => {
        self.cond.push(ast);
      },
      IfGenState::Then => {
        self.then_body.push(ast);
      },
      IfGenState::Else => {
        self.else_body.push(ast);
      },
    }
  }

  fn sep(&mut self) {
    match self.state {
      IfGenState::Cond => {
        assert!(!self.cond.is_empty());
        self.state = IfGenState::Then;
      },
      IfGenState::Then => {
        assert!(!self.then_body.is_empty());
        self.state = IfGenState::Else;
      },
      IfGenState::Else => {
        panic!()
      },
    }
  }

  fn end(&mut self) -> bool { self.state == IfGenState::Else }
}

// #[derive(Debug, Clone)]
// pub struct DoneOpMold {
//   pub inst_rule: InstRule,
// }

#[derive(Debug, Clone)]
pub struct DelayOpMold {
  pub inst_rule: InstRule,
  pub delay: u32,
}

impl OpMold for DelayOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    let res = slotmap.insert(Value::new_wo_ty(None));
    results.push(Op::delay(self.inst_rule, self.delay, res).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub struct DynDelayOpMold {
  pub inst_rule: InstRule,
  pub delay: u32,
}
impl OpMold for DynDelayOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert!(expect_ret <= 1);
    let res = slotmap.insert(Value::new_wo_ty(None));
    results.push(Op::dyn_delay(self.inst_rule, self.delay, res).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub struct AssignOpMold {
  pub res: Option<ValueMold>,
  pub value: Option<AST>,
}
impl OpMold for AssignOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let res = self.res.unwrap().id;
    let value = flatten_one!(self.value.unwrap(), slotmap, results);
    results.push(Op::assign(value, res).with_span(span, "cmtrs"));
    vec![]
  }
}

#[derive(Debug, Clone)]
pub struct MultiAssignOpMold {
  pub res: Vec<ValueMold>,
  pub value: Option<AST>,
}
impl OpMold for MultiAssignOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let res: Vec<ValueId> = self.res.into_iter().map(|x| x.id).collect();
    let value = self.value.unwrap().flatten(res.len(), slotmap, results);
    for (r, v) in res.into_iter().zip(value) {
      results.push(Op::assign(v, r).with_span(span.clone(), "cmtrs"));
    }
    vec![]
  }
}

#[derive(Debug, Clone)]
pub struct ReturnOpMold {
  pub values: Vec<AST>,
}

impl OpMold for ReturnOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    let values: Vec<ValueId> = self.values.into_iter().map(|x| flatten_one!(x, slotmap, results)).collect();
    results.push(Op::ret(values.clone()).with_span(span, "cmtrs"));
    assert!(expect_ret == 0 || expect_ret == values.len());
    values
  }
}

// #[derive(Debug, Clone)]
// pub enum FieldValueMold {
//   Inst { instance: String, wire: AST },
// }

// #[derive(Debug, Clone)]
// pub struct FieldOpMold {
//   pub value: FieldValueMold,
// }
// impl OpMold for FieldOpMold {
//   fn flatten(self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, results: &mut Vec<Op>) -> Vec<ValueId> {
//     match self.value {
//       FieldValueMold::Inst { instance, wire } => {
//         assert!(expect_ret <= 1);
//         let wire = flatten_one!(wire, slotmap, results);
//         let res = slotmap.insert(Value::new_wo_ty(None));
//         results.push(Op::Field(FieldOp {
//           res,
//           value: cmtir::FieldValue::Inst { instance, wire },
//         }));
//         vec![res]
//       },
//     }
//   }
// }

#[derive(Debug, Clone)]
pub struct StepOpMold {
  pub ops: Vec<AST>,
}

impl OpMold for StepOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let mut ops = Vec::new();
    for op in self.ops {
      op.flatten(0, slotmap, &mut ops);
    }
    results.push(Op::step(ops).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for StepOpMold {
  fn statement(&mut self, ast: AST) { self.ops.push(ast); }

  fn sep(&mut self) {}

  fn end(&mut self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct SeqOpMold {
  pub ops: Vec<AST>,
}

impl OpMold for SeqOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let mut ops = Vec::new();
    for op in self.ops {
      op.flatten(0, slotmap, &mut ops);
    }
    results.push(Op::seq(ops).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for SeqOpMold {
  fn statement(&mut self, ast: AST) { self.ops.push(ast); }

  fn sep(&mut self) {}

  fn end(&mut self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct ParOpMold {
  pub ops: Vec<AST>,
}

impl OpMold for ParOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let mut ops = Vec::new();
    for op in self.ops {
      op.flatten(0, slotmap, &mut ops);
    }
    results.push(Op::par(ops).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for ParOpMold {
  fn statement(&mut self, ast: AST) { self.ops.push(ast); }

  fn sep(&mut self) {}

  fn end(&mut self) -> bool { true }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum BranchGenState {
  Cond,
  Then,
  Else,
}

#[derive(Debug, Clone)]
pub struct BranchOpMold {
  pub cond: Vec<AST>,
  pub then_body: Option<AST>,
  pub else_body: Option<AST>,
  pub loose: bool,
  state: BranchGenState,
}

impl OpMold for BranchOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let cond = flatten_block!(self.cond, slotmap, 1);
    let then_body = flatten_op!(self.then_body.unwrap(), slotmap, 0);
    let else_body = self.else_body.map(|else_body| flatten_op!(else_body, slotmap, 0));
    results.push(Op::branch(cond, then_body, else_body, self.loose).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for BranchOpMold {
  fn statement(&mut self, ast: AST) {
    match self.state {
      BranchGenState::Cond => {
        self.cond.push(ast);
      },
      BranchGenState::Then => {
        assert!(self.then_body.is_none());
        self.then_body = Some(ast);
      },
      BranchGenState::Else => {
        assert!(self.else_body.is_none());
        self.else_body = Some(ast);
      },
    }
  }

  fn sep(&mut self) {
    match self.state {
      BranchGenState::Cond => {
        assert!(!self.cond.is_empty());
        self.state = BranchGenState::Then;
      },
      BranchGenState::Then => {
        assert!(self.then_body.is_some());
        self.state = BranchGenState::Else;
      },
      BranchGenState::Else => {
        panic!()
      },
    }
  }

  fn end(&mut self) -> bool { self.state == BranchGenState::Else }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LooseForGenState {
  Init,
  Cond,
  Update,
  Body,
}

#[derive(Debug, Clone)]
pub struct LooseForOpMold {
  // pub var: ValueMold,
  pub init: Vec<AST>,
  pub cond: Vec<AST>,
  pub update: Vec<AST>,
  pub body: Option<AST>,
  state: LooseForGenState,
}

impl OpMold for LooseForOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let init = if self.init.is_empty() { None } else { Some(flatten_block!(self.init, slotmap, 0)) };
    let cond = flatten_block!(self.cond, slotmap, 1);
    let update = flatten_block!(self.update, slotmap, 0);
    let body = flatten_op!(self.body.unwrap(), slotmap, 0);
    results.push(ir::Op::for_loose(init, cond, update, body).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for LooseForOpMold {
  fn statement(&mut self, ast: AST) {
    match self.state {
      LooseForGenState::Init => self.init.push(ast),
      LooseForGenState::Cond => self.cond.push(ast),
      LooseForGenState::Update => self.update.push(ast),
      LooseForGenState::Body => {
        assert!(self.body.is_none());
        self.body = Some(ast);
      },
    }
  }

  fn sep(&mut self) {
    match self.state {
      LooseForGenState::Init => self.state = LooseForGenState::Cond,
      LooseForGenState::Cond => {
        assert!(!self.cond.is_empty());
        self.state = LooseForGenState::Update;
      },
      LooseForGenState::Update => self.state = LooseForGenState::Body,
      LooseForGenState::Body => panic!(),
    }
  }

  fn end(&mut self) -> bool { self.state == LooseForGenState::Body }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum TightForGenState {
  Init,
  InitCond,
  Update,
  UpdateCond,
  Body,
}

#[derive(Debug, Clone)]
pub struct TightForOpMold {
  // pub var: ValueMold,
  pub init: Vec<AST>,
  pub init_cond: Vec<AST>,
  pub update: Vec<AST>,
  pub update_cond: Vec<AST>,
  pub body: Option<AST>,
  state: TightForGenState,
}

impl OpMold for TightForOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    let init = if self.init.is_empty() { None } else { Some(flatten_block!(self.init, slotmap, 0)) };
    let init_cond = flatten_block!(self.init_cond, slotmap, 1);
    let update = flatten_block!(self.update, slotmap, 0);
    let update_cond = flatten_block!(self.update_cond, slotmap, 1);
    let body = flatten_op!(self.body.unwrap(), slotmap, 0);
    results.push(Op::for_tight(init, init_cond, update, update_cond, body).with_span(span, "cmtrs"));
    Vec::new()
  }
}
impl StatementMold for TightForOpMold {
  fn statement(&mut self, ast: AST) {
    match self.state {
      TightForGenState::Init => self.init.push(ast),
      TightForGenState::InitCond => self.init_cond.push(ast),
      TightForGenState::Update => self.update.push(ast),
      TightForGenState::UpdateCond => self.update_cond.push(ast),
      TightForGenState::Body => {
        assert!(self.body.is_none());
        self.body = Some(ast);
      },
    }
  }

  fn sep(&mut self) {
    match self.state {
      TightForGenState::Init => self.state = TightForGenState::InitCond,
      TightForGenState::InitCond => self.state = TightForGenState::Update,
      TightForGenState::Update => self.state = TightForGenState::UpdateCond,
      TightForGenState::UpdateCond => {
        assert!(!self.update_cond.is_empty());
        self.state = TightForGenState::Body;
      },
      TightForGenState::Body => panic!(),
    }
  }

  fn end(&mut self) -> bool { self.state == TightForGenState::Body }
}

#[derive(Debug, Clone)]
pub struct SimPrintOpMold {
  pub ops: Vec<AST>,
  pub items: Vec<PrintItem>,
}

impl OpMold for SimPrintOpMold {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);

    let block = flatten_block!(self.ops, slotmap, 0);
    results.push(block);
    results.push(Op::sim_print(self.items).with_span(span, "cmtrs"));

    Vec::new()
  }
}

impl StatementMold for SimPrintOpMold {
  fn statement(&mut self, ast: AST) { self.ops.push(ast); }

  fn sep(&mut self) {}

  fn end(&mut self) -> bool { true }
}

impl OpMold for SimExitOp {
  fn flatten(
    self, expect_ret: usize, _slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 0);
    results.push(Op::sim_exit().with_span(span, "cmtrs"));
    Vec::new()
  }
}

#[derive(Debug, Clone)]
pub struct SimFromInt {
  pub a: AST,
  pub ty: Type,
}

impl OpMold for SimFromInt {
  fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, span: Option<MySpan>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    assert_eq!(expect_ret, 1);
    let res = slotmap.insert(Value::new(self.ty.into(), None));
    let a = flatten_one!(self.a, slotmap, results);
    results.push(Op::sim_from_int(res, a).with_span(span, "cmtrs"));
    vec![res]
  }
}

#[derive(Debug, Clone)]
pub enum ASTNode {
  Nop(NopOp),
  Assign(Box<AssignOpMold>),
  MultiAssign(Box<MultiAssignOpMold>),
  Lit(LitOpMold),
  Cmp(Box<CmpOpMold>),
  Prim(PrimOpMold),
  Invoke(InvokeOpMold),
  // Field(Box<FieldOpMold>),
  Timed(TimingIntv, Box<AST>),
  Block(Box<BodyOpMold>),
  If(Box<IfOpMold>),
  Return(Box<ReturnOpMold>),
  Delay(DelayOpMold),
  DynDelay(DynDelayOpMold),
  Value(ValueMold),
  MultiValue(MultiValueMold),
  Step(Box<StepOpMold>),
  Seq(Box<SeqOpMold>),
  Par(Box<ParOpMold>),
  Branch(Box<BranchOpMold>),
  LooseFor(Box<LooseForOpMold>),
  TightFor(Box<TightForOpMold>),
  SimPrint(SimPrintOpMold),
  SimExit(SimExitOp),
  SimFromInt(Box<SimFromInt>),
}

impl ASTNode {
  pub fn with_span(self, span: Option<MySpan>) -> AST { AST { node: self, span } }
}

#[derive(Debug, Clone)]
pub struct AST {
  pub node: ASTNode,
  pub span: Option<MySpan>,
}

impl AST {
  pub fn assign(lhs: ValueMold, rhs: AST, span: Option<MySpan>) -> AST {
    ASTNode::Assign(Box::new(AssignOpMold { res: Some(lhs), value: Some(rhs) })).with_span(span)
  }

  pub fn multi_assign(lhs: Vec<ValueMold>, rhs: AST, span: Option<MySpan>) -> AST {
    ASTNode::MultiAssign(Box::new(MultiAssignOpMold { res: lhs, value: Some(rhs) })).with_span(span)
  }

  pub fn lit(value: TypedLit, span: Option<MySpan>) -> AST { ASTNode::Lit(LitOpMold { value }).with_span(span) }

  pub fn cmp(cmp: Cmp, a: AST, b: AST, span: Option<MySpan>) -> AST {
    ASTNode::Cmp(Box::new(CmpOpMold { cmp, a: Some(a), b: Some(b) })).with_span(span)
  }

  pub fn binary(prim: Prim, a: AST, b: AST, span: Option<MySpan>) -> AST {
    ASTNode::Prim(PrimOpMold { prim, inputs: vec![a, b], attrs: vec![] }).with_span(span)
  }

  pub fn binary_w_attr(p: Prim, a: AST, b: u32, span: Option<MySpan>) -> AST {
    ASTNode::Prim(PrimOpMold { prim: p, inputs: vec![a], attrs: vec![b] }).with_span(span)
  }

  pub fn binary_w_2attr(p: Prim, a: AST, b: u32, c: u32, span: Option<MySpan>) -> AST {
    ASTNode::Prim(PrimOpMold {
      prim: p,
      inputs: vec![a],
      attrs: vec![b, c],
    })
    .with_span(span)
  }

  pub fn unary(p: Prim, a: AST, span: Option<MySpan>) -> AST {
    ASTNode::Prim(PrimOpMold { prim: p, inputs: vec![a], attrs: vec![] }).with_span(span)
  }

  pub fn block(ops: Vec<AST>, span: Option<MySpan>) -> AST {
    ASTNode::Block(Box::new(BodyOpMold { ops })).with_span(span)
  }

  pub fn if_mold(span: Option<MySpan>) -> AST {
    ASTNode::If(Box::new(IfOpMold {
      cond: Vec::new(),
      then_body: Vec::new(),
      else_body: Vec::new(),
      state: IfGenState::Cond,
    }))
    .with_span(span)
  }

  pub fn invoke(path: Vec<String>, rule_name: String, args: Vec<AST>, num_res: usize, span: Option<MySpan>) -> AST {
    ASTNode::Invoke(InvokeOpMold {
      inst_rule: InstRule { path, rule_name },
      args,
      num_res,
    })
    .with_span(span)
  }

  pub fn ret(values: Vec<AST>, span: Option<MySpan>) -> AST {
    ASTNode::Return(Box::new(ReturnOpMold { values })).with_span(span)
  }

  pub fn value(id: ValueId, span: Option<MySpan>) -> AST { ASTNode::Value(ValueMold { id }).with_span(span) }

  pub fn step_mold(span: Option<MySpan>) -> AST {
    ASTNode::Step(Box::new(StepOpMold { ops: Vec::new() })).with_span(span)
  }

  pub fn seq_mold(span: Option<MySpan>) -> AST { ASTNode::Seq(Box::new(SeqOpMold { ops: Vec::new() })).with_span(span) }

  pub fn par_mold(span: Option<MySpan>) -> AST { ASTNode::Par(Box::new(ParOpMold { ops: Vec::new() })).with_span(span) }

  pub fn branch_mold(loose: bool, span: Option<MySpan>) -> AST {
    ASTNode::Branch(Box::new(BranchOpMold {
      cond: Vec::new(),
      then_body: None,
      else_body: None,
      loose,
      state: BranchGenState::Cond,
    }))
    .with_span(span)
  }

  pub fn for_mold(loose: bool, span: Option<MySpan>) -> AST {
    if loose {
      ASTNode::LooseFor(Box::new(LooseForOpMold {
        init: Vec::new(),
        cond: Vec::new(),
        update: Vec::new(),
        body: None,
        state: LooseForGenState::Init,
      }))
    } else {
      ASTNode::TightFor(Box::new(TightForOpMold {
        init: Vec::new(),
        init_cond: Vec::new(),
        update: Vec::new(),
        update_cond: Vec::new(),
        body: None,
        state: TightForGenState::Init,
      }))
    }
    .with_span(span)
  }

  pub fn print_mold(span: Option<MySpan>) -> AST {
    ASTNode::SimPrint(SimPrintOpMold { ops: Vec::new(), items: Vec::new() }).with_span(span)
  }

  pub fn sim_exit(span: Option<MySpan>) -> AST { ASTNode::SimExit(SimExitOp {}).with_span(span) }

  pub fn sim_from_int(a: AST, ty: Type, span: Option<MySpan>) -> AST {
    ASTNode::SimFromInt(Box::new(SimFromInt { a, ty })).with_span(span)
  }

  pub fn flatten(
    self, expect_ret: usize, slotmap: &mut SlotMap<ValueId, Value>, results: &mut Vec<Op>,
  ) -> Vec<ValueId> {
    match self.node {
      ASTNode::Nop(op) => {
        results.push(OpEnum::Nop(op).into());
        vec![]
      },
      ASTNode::Assign(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::MultiAssign(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Lit(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Cmp(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Prim(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Invoke(op) => op.flatten(expect_ret, slotmap, self.span, results),
      // AST::Field(op) => op.flatten(expect_ret, slotmap, results),
      ASTNode::Timed(interval, op) => {
        let mut ops = Vec::new();
        let ret = op.flatten(expect_ret, slotmap, &mut ops);
        assert!(ret.is_empty());
        assert_eq!(ops.len(), 1);
        let op = ops.into_iter().next().unwrap();
        results.push(Op::timed(interval, op).with_span(self.span, "cmtrs"));
        vec![]
      },
      ASTNode::Block(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::If(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Return(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Delay(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::DynDelay(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Value(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::MultiValue(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Step(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Seq(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Par(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::Branch(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::LooseFor(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::TightFor(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::SimPrint(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::SimExit(op) => op.flatten(expect_ret, slotmap, self.span, results),
      ASTNode::SimFromInt(op) => op.flatten(expect_ret, slotmap, self.span, results),
    }
  }

  fn statement(&mut self, ast: AST) {
    match &mut self.node {
      ASTNode::Block(op) => op.statement(ast),
      ASTNode::If(op) => op.statement(ast),
      ASTNode::Step(op) => op.statement(ast),
      ASTNode::Seq(op) => op.statement(ast),
      ASTNode::Par(op) => op.statement(ast),
      ASTNode::Branch(op) => op.statement(ast),
      ASTNode::LooseFor(op) => op.statement(ast),
      ASTNode::TightFor(op) => op.statement(ast),
      ASTNode::SimPrint(op) => op.statement(ast),
      _ => panic!("call statement() on {:?}", self),
    }
  }

  fn sep(&mut self) {
    match &mut self.node {
      ASTNode::Block(op) => op.sep(),
      ASTNode::If(op) => op.sep(),
      ASTNode::Step(op) => op.sep(),
      ASTNode::Seq(op) => op.sep(),
      ASTNode::Par(op) => op.sep(),
      ASTNode::Branch(op) => op.sep(),
      ASTNode::LooseFor(op) => op.sep(),
      ASTNode::TightFor(op) => op.sep(),
      ASTNode::SimPrint(op) => op.sep(),
      _ => panic!("call sep() on {:?}", self),
    }
  }

  fn end(&mut self) -> bool {
    match &mut self.node {
      ASTNode::Block(op) => op.end(),
      ASTNode::If(op) => op.end(),
      ASTNode::Step(op) => op.end(),
      ASTNode::Seq(op) => op.end(),
      ASTNode::Par(op) => op.end(),
      ASTNode::Branch(op) => op.end(),
      ASTNode::LooseFor(op) => op.end(),
      ASTNode::TightFor(op) => op.end(),
      ASTNode::SimPrint(op) => op.end(),
      _ => panic!("call end() on {:?}", self),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RuleGenState {
  Guard,
  Body,
}

#[derive(Debug, Clone)]
pub struct RuleMold {
  is_ext: bool,
  is_private: bool,
  name: String,
  signature: RuleSignature,
  timing: RuleTiming,
  enable: Option<String>,
  ready: Option<String>,
  guard_ops: Vec<Op>,
  ops: Vec<Op>,
  gen_state: RuleGenState,
  span: Option<MySpan>,
}

impl RuleMold {
  pub fn new(
    is_ext: bool, is_private: bool, name: String, signature: RuleSignature, timing: RuleTiming, enable: Option<String>,
    ready: Option<String>, span: Option<MySpan>,
  ) -> Self {
    Self {
      is_ext,
      is_private,
      name,
      signature,
      timing,
      enable,
      ready,
      guard_ops: Vec::new(),
      ops: Vec::new(),
      gen_state: RuleGenState::Guard,
      span,
    }
  }

  pub fn rule_guard(&mut self, guard: Option<AST>, slotmap: &mut SlotMap<ValueId, Value>) {
    debug_assert_eq!(self.gen_state, RuleGenState::Guard);
    if let Some(guard) = guard {
      self.push_op(AST::ret(vec![guard], None), slotmap);
    }
    self.gen_state = RuleGenState::Body;
  }

  pub fn push_op(&mut self, op: AST, slotmap: &mut SlotMap<ValueId, Value>) {
    let mut ops = Vec::new();
    let _ = op.flatten(0, slotmap, &mut ops);
    match self.gen_state {
      RuleGenState::Guard => self.guard_ops.extend(ops),
      RuleGenState::Body => self.ops.extend(ops),
    }
  }

  pub fn make(self) -> Rule {
    debug_assert_eq!(self.gen_state, RuleGenState::Body);
    let mut annotations = json::object::Object::new();
    if let Some(span) = self.span {
      annotations.insert("cmtrs_span", span.into());
    }
    Rule {
      is_ext: self.is_ext,
      is_private: self.is_private,
      name: self.name,
      signature: self.signature,
      timing: self.timing,
      enable: self.enable,
      ready: self.ready,
      guard_ops: self.guard_ops,
      ops: self.ops,
      annotations,
    }
  }

  pub fn get_name(&self) -> &str { &self.name }
}

#[derive(Debug, Clone)]
pub struct ModuleMold {
  name: Option<String>,
  values: Option<SlotMap<ValueId, Value>>,
  annotations: IndexMap<String, String>,
  inputs: IndexMap<String, ValueId>,
  outputs: IndexMap<String, ValueId>,
  sub_modules: IndexMap<String, Module>,
  instance_cnt: IndexMap<String, usize>,
  instances: Vec<InstDef>,
  body: Option<ModuleBody>,
  rules: IndexMap<String, Rule>,
  current_rule: Option<RuleMold>,
  rule_rels: Vec<RuleRel>,
  ast_stack: Vec<AST>,
  var_names: IndexMap<String, usize>,
  span: Option<MySpan>,
  error: CmtError,
}

impl ModuleMold {
  pub fn new(name: String, span: Option<MySpan>) -> Self {
    ModuleMold {
      name: Some(name),
      values: Some(SlotMap::default()),
      annotations: IndexMap::new(),
      inputs: IndexMap::new(),
      outputs: IndexMap::new(),
      sub_modules: IndexMap::new(),
      instance_cnt: IndexMap::new(),
      instances: Vec::new(),
      body: None,
      rules: IndexMap::new(),
      current_rule: None,
      rule_rels: Vec::new(),
      ast_stack: Vec::new(),
      var_names: IndexMap::new(),
      span,
      error: CmtError::new(),
    }
  }

  pub fn clone_with_new_name(&self, name: String) -> Self { Self { name: Some(name), ..self.clone() } }

  pub fn set_name(&mut self, name: String) { self.name = Some(name); }

  pub fn distinguisher(&mut self, distinguish_str: &str) {
    self.name = Some(format!("{}_{}", self.name.as_ref().unwrap(), distinguish_str));
  }

  pub fn get_name(&self) -> &str { self.name.as_ref().unwrap() }

  pub fn make_modules(&mut self) -> Vec<Module> {
    let inputs = self.inputs.clone();
    let values = self.values.clone();
    let outputs = self.outputs.clone();
    let rules = self.rules.clone();
    let mut result: Vec<Module> = self.sub_modules.drain(..).map(|(_, m)| m).collect();

    let mut annotations = json::object::Object::new();
    for (key, value) in self.annotations.drain(..) {
      annotations.insert(&key, json::JsonValue::String(value));
    }

    if let Some(span) = self.span.clone() {
      annotations.insert("cmtrs_span", span.into());
    }

    result.push(Module {
      name: self.name.take().unwrap(),
      values: values.unwrap(),
      annotations,
      inputs: inputs.into_iter().map(|(_, i)| i).collect(),
      outputs: outputs.into_iter().map(|(_, o)| o).collect(),
      wires: Vec::new(),
      body: self.body.take().unwrap(),
      rules: rules.into_iter().map(|(_, r)| r).collect(),
      rule_rels: self.rule_rels.drain(..).collect(),
    });

    result
  }

  pub fn add_var(&mut self, name: Option<String>) -> Var {
    let name = name.map(|name| {
      let c = self.var_names.entry(name.clone()).or_default();
      *c += 1;
      if *c != 1 {
        let mut name = format!("{}_{}", name, c);
        while self.var_names.contains_key(&name) {
          let c = self.var_names.entry(name.clone()).or_default();
          *c += 1;
          name = format!("{}_{}", name, c);
        }
        name
      } else {
        name
      }
    });
    let id = self.values.as_mut().unwrap().insert(Value::new_wo_ty(name));
    Var::new_val(id)
  }

  pub fn inputs(&self) -> Vec<(ValueId, String, Type)> {
    self
      .inputs
      .iter()
      .map(|(_, x)| {
        (
          *x,
          self.values.as_ref().unwrap().get(*x).unwrap().name.clone().unwrap(),
          self.values.as_ref().unwrap().get(*x).unwrap().ty.clone().unwrap().into(),
        )
      })
      .collect()
  }

  pub fn outputs(&self) -> Vec<(ValueId, String, Type)> {
    self
      .outputs
      .iter()
      .map(|(_, x)| {
        (
          *x,
          self.values.as_ref().unwrap().get(*x).unwrap().name.clone().unwrap(),
          self.values.as_ref().unwrap().get(*x).unwrap().ty.clone().unwrap().into(),
        )
      })
      .collect()
  }

  #[track_caller]
  pub fn add_input(&mut self, name: String, ty: Type) -> Var {
    if self.inputs.contains_key(&name) {
      self.error.append("Duplicated input names".to_string(), Some(extract_span_from_location(Location::caller())));
    }
    let id = self.values.as_mut().unwrap().insert(Value::new(ty.into(), Some(name.clone())));
    self.inputs.insert(name, id);
    Var::new_val(id)
  }

  #[track_caller]
  pub fn add_output(&mut self, name: String, ty: Type) -> Var {
    if self.outputs.contains_key(&name) {
      self.error.append("Duplicated output names".to_string(), Some(extract_span_from_location(Location::caller())));
    }
    let id = self.values.as_mut().unwrap().insert(Value::new(ty.into(), Some(name.clone())));
    self.outputs.insert(name, id);
    Var::new_val(id)
  }

  pub fn add_submodule(&mut self, sub_mod: Module) {
    if self.sub_modules.contains_key(sub_mod.name()) {
      log::warn!(
        "\tPotentially duplicated sub-module with name {} in module {}",
        sub_mod.name(),
        self.name.as_ref().unwrap()
      );
    }
    self.sub_modules.insert(sub_mod.name.clone(), sub_mod);
  }

  pub fn instance(&mut self, mut inst: InstDef) -> String {
    let e = self.instance_cnt.entry(inst.name.clone()).or_default();
    *e += 1;
    if *e != 1 {
      let mut name = format!("{}_{}", inst.name, *e);
      while self.instance_cnt.contains_key(&name) {
        let c = self.instance_cnt.entry(name.clone()).or_default();
        *c += 1;
        name = format!("{}_{}", name, *c);
      }
      inst = InstDef { name, module: inst.module };
    }
    let name = inst.name.clone();
    self.instances.push(inst);
    name
  }

  pub fn add_annotation(&mut self, field: String, value: String) { self.annotations.insert(field, value); }

  pub fn begin_rule(
    &mut self, is_ext: bool, is_private: bool, name: String, signature: RuleSignature, timing: RuleTiming,
    enable: Option<String>, ready: Option<String>, span: Option<MySpan>,
  ) {
    // log::info!("begin_rule at {:?}", span);
    self.current_rule = Some(RuleMold::new(is_ext, is_private, name, signature, timing, enable, ready, span));
  }

  pub fn rule_guard(&mut self, guard: Option<AST>) {
    if let Some(rule_mold) = &mut self.current_rule {
      rule_mold.rule_guard(guard, &mut self.values.as_mut().unwrap());
    } else {
      panic!("rule_guard without rule");
    }
  }

  pub fn end_rule(&mut self) -> RuleHandle {
    let cur_rule_mold = self.current_rule.take().unwrap();
    if self.rules.contains_key(&cur_rule_mold.name) {
      self.error.append("Duplicated rule name".to_string(), cur_rule_mold.span.clone());
    }
    self.rules.insert(cur_rule_mold.name.clone(), cur_rule_mold.make());
    RuleHandle::new(RuleId(self.rules.len() - 1))
  }

  pub fn pub_method_signatures(&self) -> Vec<(String, RuleSignature)> {
    self
      .rules
      .iter()
      .filter(|(_, r)| r.is_method() && !r.is_private())
      .map(|(_, r)| (r.name.clone(), r.signature.clone()))
      .collect()
  }

  pub fn statement(&mut self, ast: AST) {
    if self.ast_stack.is_empty() {
      if let Some(rule_mold) = &mut self.current_rule {
        rule_mold.push_op(ast, &mut self.values.as_mut().unwrap());
      } else {
        panic!("Statement outside of a rule");
      }
    } else {
      self.ast_stack.last_mut().unwrap().statement(ast);
    }
  }

  pub fn ret(&mut self, values: Vec<AST>, span: Option<MySpan>) { self.statement(AST::ret(values, span)); }

  pub fn assign(&mut self, lhs: ValueId, rhs: AST, span: Option<MySpan>) {
    self.statement(AST::assign(ValueMold { id: lhs }, rhs, span));
  }

  pub fn multi_assign(&mut self, lhs: Vec<ValueId>, rhs: AST, span: Option<MySpan>) {
    self.statement(AST::multi_assign(lhs.into_iter().map(|id| ValueMold { id }).collect(), rhs, span));
  }

  pub fn block(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::block(Vec::new(), span)); }

  pub fn if_(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::if_mold(span)); }

  pub fn step(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::step_mold(span)); }

  pub fn seq(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::seq_mold(span)); }

  pub fn par(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::par_mold(span)); }

  pub fn branch(&mut self, loose: bool, span: Option<MySpan>) { self.ast_stack.push(AST::branch_mold(loose, span)); }

  pub fn for_(&mut self, loose: bool, span: Option<MySpan>) { self.ast_stack.push(AST::for_mold(loose, span)); }

  pub fn print(&mut self, span: Option<MySpan>) { self.ast_stack.push(AST::print_mold(span)); }

  pub fn print_item(&mut self, item: PrintItem) {
    if let Some(AST { node: ASTNode::SimPrint(s), .. }) = self.ast_stack.last_mut() {
      s.items.push(item);
    } else {
      panic!("Called print_item outside of a print")
    }
  }

  pub fn sep(&mut self) { self.ast_stack.last_mut().unwrap().sep(); }

  pub fn end(&mut self) -> AST {
    assert!(self.ast_stack.last_mut().unwrap().end());
    self.ast_stack.pop().unwrap()
  }

  pub fn invoke(
    &mut self, inst_path: Vec<String>, rule_name: String, args: Vec<AST>, num_res: usize, span: Option<MySpan>,
  ) -> Vec<Var> {
    let ast = AST::invoke(inst_path, rule_name, args, num_res, span.clone());
    if num_res == 0 {
      self.statement(ast);
      Vec::new()
    } else {
      let vars: Vec<Var> = (0..num_res).map(|_| self.add_var(None)).collect();
      let vals = vars.iter().map(|v| v.value_id().unwrap()).collect();
      self.multi_assign(vals, ast, span);
      vars
    }
  }

  pub fn external(&mut self, ext_body: ExtModuleBody) { self.body = Some(ModuleBody::Ext(ext_body)); }

  pub fn method_rel(&mut self, rel: MethodRel, lhs: &[RuleHandle], rhs: &[RuleHandle]) {
    self.rule_rels.push(RuleRel::method(
      rel,
      lhs.iter().map(|i| InstRule::new(vec![self.rules[i.id().0].name.clone()]).canonicalize()).collect(),
      rhs.iter().map(|i| InstRule::new(vec![self.rules[i.id().0].name.clone()]).canonicalize()).collect(),
    ));
  }

  pub fn schedule(&mut self, order: &[RuleHandle]) {
    self
      .rule_rels
      .push(RuleRel::Schedule(order.iter().map(|i| InstRule::from_str(&self.rules[i.id().0].name)).collect()))
  }

  pub fn sim_exit(&mut self, span: Option<MySpan>) { self.statement(AST::sim_exit(span)); }

  pub fn finalize(&mut self, methods: &[&str]) {
    assert!(self.name.is_some(), "Module needs a name");

    let mut s: HashSet<&str> = methods.iter().copied().collect();
    for (_, r) in &self.rules {
      s.remove(r.name());
    }
    if !s.is_empty() {
      panic!("Module {} does not impl methods: [{}]", self.name.as_ref().unwrap(), s.iter().join(","));
    }

    assert!(
      self.current_rule.is_none(),
      "Module {} rule {} does not end appropriately",
      self.name.as_ref().unwrap(),
      self.current_rule.as_ref().unwrap().name
    );

    if self.body.is_none() {
      self.body = Some(ModuleBody::Native(NativeModuleBody {
        instances: self.instances.drain(..).collect(),
      }))
    }

    if !self.error.messages.is_empty() {
      self.error.print();
      panic!("Compile error");
    }
  }
}
