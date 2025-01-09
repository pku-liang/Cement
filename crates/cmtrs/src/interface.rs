//! Cmtrs interface for both generators and users

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::panic::Location;
use std::rc::{Rc, Weak};

use {crate as cmtrs, cmtir as ir};

use crate::extract_span_from_location;
use crate::gen_ir::*;
use crate::type_sys::*;

pub mod ops;
use cmtir::MySpan;
pub use num::{BigInt, BigUint};
pub use ops::*;

#[doc(hidden)]
pub trait Itfc: Sized + 'static {
  // Required
  type IO: ItfcIO;
  type Params: ItfcParams;
  type Inst: Instance<Params = Self::Params>;

  fn as_context(params: Self::Params, span: Option<MySpan>) -> Self;
  fn module_mut(&mut self) -> &mut ModuleMold;
  fn module(&self) -> &ModuleMold;
  fn methods(&self) -> &'static [&'static str];
  fn params(&self) -> &Self::Params;
  fn io(ctx: ContextItfc<Self>) -> Self::IO;
  // fn instantiate_subs(ctx: &ContextItfc<Self>);
  fn into_inner(self) -> (ModuleMold, Self::Params);

  // Provided
  fn dump(ctx: ContextItfc<Self>) -> Self::Inst {
    let mut itfc = ctx.take().unwrap();
    let methods = itfc.methods();
    itfc.module_mut().finalize(methods);
    let (module, params) = itfc.into_inner();
    Self::Inst::from_itfc(module, params)
  }

  fn set_name(ctx: &ContextItfc<Self>, name: String) { ctx.borrow_mut().as_mut().unwrap().module_mut().set_name(name); }

  fn distinguisher(ctx: &ContextItfc<Self>, distinguish_str: &str) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().distinguisher(distinguish_str);
  }

  fn get_name(&self) -> &str { self.module().get_name() }

  #[track_caller]
  fn add_input(ctx: &ContextItfc<Self>, name: String, ty: Type) -> Var {
    ctx.borrow_mut().as_mut().unwrap().module_mut().add_input(name, ty)
  }

  #[track_caller]
  fn add_output(ctx: &ContextItfc<Self>, name: String, ty: Type) -> Var {
    ctx.borrow_mut().as_mut().unwrap().module_mut().add_output(name, ty)
  }

  #[track_caller]
  fn add_var(ctx: &ContextItfc<Self>, name: Option<String>) -> Var {
    ctx.borrow_mut().as_mut().unwrap().module_mut().add_var(name)
  }

  #[track_caller]
  fn add_annotation(ctx: &ContextItfc<Self>, field: &str, value: &str) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().add_annotation(field.to_string(), value.to_string());
  }

  fn begin_rule(
    ctx: &ContextItfc<Self>, is_ext: bool, is_private: bool, name: String, signature: RuleSignature,
    timing: RuleTiming, enable: Option<String>, ready: Option<String>, span: Option<MySpan>,
  ) {
    ctx
      .borrow_mut()
      .as_mut()
      .unwrap()
      .module_mut()
      .begin_rule(is_ext, is_private, name, signature, timing, enable, ready, span);
  }

  fn rule_guard(ctx: &ContextItfc<Self>) { ctx.borrow_mut().as_mut().unwrap().module_mut().rule_guard(None); }

  fn rule_guard_var(ctx: &ContextItfc<Self>, guard: impl CmtAST) {
    let ast = Some(guard.ast());
    ctx.borrow_mut().as_mut().unwrap().module_mut().rule_guard(ast);
  }

  fn end_rule(ctx: &ContextItfc<Self>) -> RuleHandle { ctx.borrow_mut().as_mut().unwrap().module_mut().end_rule() }

  /// Assign rhs to lhs, this method will call [statement](`Itfc::statement()`) implicitly
  fn assign(ctx: &ContextItfc<Self>, lhs: &Var, rhs: impl CmtAST, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().assign(lhs.value_id().unwrap(), rhs.ast(), span);
  }
  fn multi_assign(ctx: &ContextItfc<Self>, lhs: &[&Var], rhs: impl CmtAST, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().multi_assign(
      lhs.iter().map(|x| x.value_id().unwrap()).collect(),
      rhs.ast(),
      span,
    );
  }

  fn statement(ctx: &ContextItfc<Self>, ast: impl CmtAST) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().statement(ast.ast());
  }
  fn ret(ctx: &ContextItfc<Self>, values: Vec<AST>, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().ret(values, span);
  }

  fn if_(ctx: &ContextItfc<Self>, span: Option<MySpan>) { ctx.borrow_mut().as_mut().unwrap().module_mut().if_(span); }
  fn block(ctx: &ContextItfc<Self>, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().block(span);
  }

  fn step(ctx: &ContextItfc<Self>, span: Option<MySpan>) { ctx.borrow_mut().as_mut().unwrap().module_mut().step(span); }
  fn seq(ctx: &ContextItfc<Self>, span: Option<MySpan>) { ctx.borrow_mut().as_mut().unwrap().module_mut().seq(span); }
  fn par(ctx: &ContextItfc<Self>, span: Option<MySpan>) { ctx.borrow_mut().as_mut().unwrap().module_mut().par(span); }
  fn branch(ctx: &ContextItfc<Self>, loose: bool, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().branch(loose, span);
  }
  fn for_(ctx: &ContextItfc<Self>, loose: bool, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().for_(loose, span);
  }

  fn sep(ctx: &ContextItfc<Self>) { ctx.borrow_mut().as_mut().unwrap().module_mut().sep(); }
  fn end(ctx: &ContextItfc<Self>) -> AST { ctx.borrow_mut().as_mut().unwrap().module_mut().end() }

  #[track_caller]
  fn instantiate<T: Instance>(ctx: &ContextItfc<Self>, name: String, mut sub_mod: T) -> T {
    let name = module_add_instance(ctx.borrow_mut().as_mut().unwrap().module_mut(), name.clone(), &mut sub_mod);

    let views = Itfc::make_views(ctx, name, Instance::nested_names(&sub_mod));
    sub_mod.as_instance(&mut views.into_iter());
    sub_mod
  }

  fn make_views(ctx: &ContextItfc<Self>, name: String, nested_names: Vec<String>) -> Vec<Box<dyn InstanceView>> {
    let mut views: Vec<Box<dyn InstanceView>> = vec![Box::new(Itfc::view(ctx, vec![name.clone()]))];
    for nested in nested_names {
      views.push(Box::new(Itfc::view(ctx, vec![name.clone(), nested])));
    }
    views
  }

  #[track_caller]
  fn invoke(
    ctx: &ContextItfc<Self>, inst_path: Vec<String>, rule_name: String, args: Vec<AST>, num_res: usize,
    span: Option<MySpan>,
  ) -> Vec<Var> {
    ctx.borrow_mut().as_mut().unwrap().module_mut().invoke(inst_path, rule_name, args, num_res, span)
  }

  fn external(
    ctx: &ContextItfc<Self>, inputs: Vec<String>, outputs: Vec<String>, clock: Option<String>, reset: Option<String>,
    fir_str: String,
  ) {
    ctx
      .borrow_mut()
      .as_mut()
      .unwrap()
      .module_mut()
      .external(ir::ExtModuleBody::new(inputs, outputs, clock, reset, fir_str));
  }

  fn method_rel(ctx: &ContextItfc<Self>, rel: MethodRel, lhs: &[RuleHandle], rhs: &[RuleHandle]) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().method_rel(rel, lhs, rhs);
  }

  fn schedule(ctx: &ContextItfc<Self>, order: &[RuleHandle]) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().schedule(order);
  }

  fn print(ctx: &ContextItfc<Self>, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().print(span);
  }
  fn print_item(ctx: &ContextItfc<Self>, item: &impl ToPrintItem) {
    let item = item.to_print_item(ctx.clone());
    ctx.borrow_mut().as_mut().unwrap().module_mut().print_item(item);
  }
  fn sim_exit(ctx: &ContextItfc<Self>, span: Option<MySpan>) {
    ctx.borrow_mut().as_mut().unwrap().module_mut().sim_exit(span);
  }

  fn view(ctx: &ContextItfc<Self>, inst_path: Vec<String>) -> InstanceViewStruct<Self> {
    InstanceViewStruct::from_rc(ctx, inst_path)
  }

  fn view_weak(ctx: Weak<RefCell<Option<Self>>>, inst_path: Vec<String>) -> InstanceViewStruct<Self> {
    InstanceViewStruct::from_weak(ctx, inst_path)
  }
}

pub type ContextItfc<T> = Rc<RefCell<Option<T>>>;

pub trait Instance: ToIOMacro<Output = Self> + Sized + 'static {
  // Required
  type Params: ItfcParams;

  fn type_name() -> &'static str;
  fn from_itfc(module: ModuleMold, params: Self::Params) -> Self;
  fn as_instance(&mut self, views: &mut InstanceViewIter);
  fn as_nested_itfc_io(&self, name: String, views: &mut InstanceViewIter) -> Self;
  fn make_nested_methods(&self, name: String, __cmt_gen: ContextItfc<impl Itfc>);
  fn module_mut(&mut self) -> &mut ModuleMold;
  fn module(&self) -> &ModuleMold;
  fn nested_names(&self) -> Vec<String>;
  fn invoke(&self, name: &str, args: &[&Var], num_res: usize, span: Option<MySpan>) -> Vars;

  // provided
  fn get_name(&self) -> &str { self.module().get_name() }

  fn set_name(&mut self, name: String) { self.module_mut().set_name(name); }

  fn make_modules(&mut self) -> Vec<Module> { self.module_mut().make_modules() }

  fn add_generated_io_methods(
    &self, prefix_name: String, exist_ins: HashSet<String>, exist_outs: HashSet<String>, exist_meths: HashSet<String>,
    __cmt_gen: cmtrs::ContextItfc<impl Itfc>,
  ) {
    let mut in_map = HashMap::new();
    let mut out_map = HashMap::new();
    for (id, name, ty) in self.module().inputs() {
      if !exist_ins.contains(&name) {
        let io = crate::input!(format!("{}_{}", prefix_name, name), ty);
        in_map.insert(id, io);
      }
    }
    for (id, name, ty) in self.module().outputs() {
      if !exist_outs.contains(&name) {
        let io = crate::output!(format!("{}_{}", prefix_name, name), ty);
        out_map.insert(id, io);
      }
    }
    for (name, signature) in self.module().pub_method_signatures() {
      if !exist_meths.contains(&name) {
        let RuleSignature::Method { inputs, outputs, side_effect } = signature else {
          panic!();
        };

        let inputs: Vec<&Var> = inputs.into_iter().map(|i| &in_map[&i]).collect();
        let outputs: Vec<&Var> = outputs.into_iter().map(|i| &out_map[&i]).collect();

        Itfc::begin_rule(
          &__cmt_gen,
          false,
          false,
          format!("{}_{}", prefix_name, name),
          RuleSignature::Method {
            inputs: inputs.iter().map(|x| x.value_id().unwrap()).collect(),
            outputs: outputs.iter().map(|x| x.value_id().unwrap()).collect(),
            side_effect,
          },
          RuleTiming::SingleCycle,
          None,
          None,
          None, // FIXME: span here?
        );
        Itfc::rule_guard(&__cmt_gen);

        self.invoke(&name, &Vec::from_iter(inputs.into_iter().map(|x| x)), outputs.len(), None);

        Itfc::end_rule(&__cmt_gen);
      }
    }
  }

  fn to_cmtir(mut self) -> ir::Circuit {
    let top_module_name = self.module().get_name().to_string();
    let mut modules = self.module_mut().make_modules();

    if let Some(last_module_mut) = modules.last_mut() {
      last_module_mut.set_top();
    }

    let mut circuit = ir::Circuit::new(top_module_name, Vec::new());
    for module in modules {
      circuit.add_module(module);
    }
    circuit
  }
}

#[doc(hidden)]
pub fn module_add_instance<T: Instance>(module: &mut ModuleMold, name: String, sub_mod: &mut T) -> String {
  let module_name = sub_mod.module().get_name().to_string();
  let mods = sub_mod.make_modules();
  for m in mods {
    module.add_submodule(m);
  }
  module.instance(ir::InstDef::new(name, module_name))
}

#[doc(hidden)]
pub trait ItfcIO {}
#[doc(hidden)]
pub trait ItfcParams {
  /// Identify this set of params as an appended string after the module name
  fn id_str(&self) -> String;
}

#[doc(hidden)]
pub trait InstanceView {
  fn invoke(&self, rule_name: String, args: Vec<AST>, num_res: usize, span: Option<MySpan>) -> Vec<Var>;
}
pub type InstanceViewIter = std::vec::IntoIter<Box<dyn InstanceView>>;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct InstanceViewStruct<ItfcTy, IOTy = <ItfcTy as Itfc>::IO, ParamsTy = <ItfcTy as Itfc>::Params>
where ItfcTy: Itfc<IO = IOTy, Params = ParamsTy>
{
  inst_path: Vec<String>,
  ctx: Weak<RefCell<Option<ItfcTy>>>,
}
impl<ItfcTy, IOTy, ParamsTy> InstanceViewStruct<ItfcTy, IOTy, ParamsTy>
where ItfcTy: Itfc<IO = IOTy, Params = ParamsTy>
{
  pub fn new() -> Self {
    InstanceViewStruct {
      inst_path: Vec::new(),
      ctx: Weak::default(),
    }
  }

  pub fn from_rc(ctx: &Rc<RefCell<Option<ItfcTy>>>, inst_path: Vec<String>) -> Self {
    InstanceViewStruct { inst_path, ctx: Rc::downgrade(ctx) }
  }

  pub fn from_weak(ctx: Weak<RefCell<Option<ItfcTy>>>, inst_path: Vec<String>) -> Self {
    InstanceViewStruct { inst_path, ctx }
  }
}

impl<ItfcTy, IOTy, ParamsTy> InstanceView for InstanceViewStruct<ItfcTy, IOTy, ParamsTy>
where ItfcTy: Itfc<IO = IOTy, Params = ParamsTy>
{
  #[track_caller]
  fn invoke(&self, rule_name: String, args: Vec<AST>, num_res: usize, span: Option<MySpan>) -> Vec<Var> {
    // let ctx = self.ctx.upgrade();
    // if ctx.is_none() {

    // }
    Itfc::invoke(
      &self.ctx.upgrade().expect("Module instance view can only be used as a sub-module in a module generator. Check if you forget to use the `instance!` macro."),
      self.inst_path.clone(),
      rule_name,
      args,
      num_res,
      span,
    )
  }
}

/// An variable in hardware
#[derive(Debug, Clone)]
pub struct Var {
  ast: AST,
}

impl Var {
  pub fn new(ast: AST) -> Self { Var { ast } }

  pub fn new_val(id: ValueId) -> Self { Var { ast: AST::value(id, None) } }

  pub fn read(&self) -> AST { self.ast.clone() }

  pub fn value_id(&self) -> Option<ValueId> {
    if let ASTNode::Value(value) = &self.ast.node {
      Some(value.id)
    } else {
      None
    }
  }
}

/// Things that can be converted into AST in Cement
/// Commonly used in method parameters so that it can receive any value in hardware
pub trait CmtAST {
  fn ast(self) -> AST;
}

impl CmtAST for Var {
  fn ast(self) -> AST { self.ast }
}
impl CmtAST for &Var {
  fn ast(self) -> AST { self.read() }
}
impl CmtAST for AST {
  fn ast(self) -> AST { self }
}

/// A vector vars, used in return values with arbitary numbers
pub struct Vars(pub Vec<Var>);
impl Vars {
  pub fn as_var(self) -> Option<Var> {
    if self.0.len() == 1 {
      Some(self.0.into_iter().next().unwrap())
    } else {
      None
    }
  }
}

impl CmtAST for Vars {
  fn ast(self) -> AST {
    assert!(self.0.len() == 1);
    self.0.into_iter().next().unwrap().ast()
  }
}

impl CmtAST for &Vars {
  fn ast(self) -> AST {
    assert!(self.0.len() == 1);
    self.0.iter().next().unwrap().ast()
  }
}

/// A handle to refer to a rule. Can be used in [`method_rel!`] and [`schedule!`]
#[derive(Debug, Clone, Copy)]
pub struct RuleHandle {
  id: RuleId,
}

impl RuleHandle {
  pub fn new(id: RuleId) -> Self { RuleHandle { id } }

  pub fn id(&self) -> RuleId { self.id }
}

/// Make a literal of certain type. May use methods from trait [`CmtLit`] as shorter replacement.
#[track_caller]
pub fn literal(a: impl Into<cmtir::RadixIntLit>, ty: &Type) -> Var {
  // get the position where the function is called
  let location = Location::caller();
  let span = extract_span_from_location(location);
  Var::new(AST::lit(TypedLit::new(a.into(), ty.clone().into()), Some(span)))
}

pub trait CmtLit: Sized {
  // required

  /// Literal of type t
  fn lit(self, t: &Type) -> Var;

  fn name(&self) -> String;

  // provided

  /// Literal of Type::UInt(w)
  fn uint(self, w: u32) -> Var { self.lit(&Type::UInt(w)) }

  /// Literal of Type::SInt(w)
  fn sint(self, w: u32) -> Var { self.lit(&Type::SInt(w)) }

  /// Literal of Type::Integer for simulation
  fn int(self) -> Var { self.lit(&Type::Integer) }
}

impl TryFrom<&str> for Var {
  type Error = String;

  #[track_caller]
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    let lit = cmtir::RadixIntLit::from_str(value)?;
    let ty = ir::Type::UInt(lit.binary_len() as u32);
    Ok(Var::new(AST::lit(TypedLit::new(lit, ty), None)))
  }
}

macro_rules! make_from_primitive {
  ($($t:ty),*) => {
    $(
      impl From<$t> for Var {
        #[track_caller]
        fn from(value: $t) -> Self {
          Var::new(AST::lit(value.into(), None))
        }
      }
      impl CmtAST for $t {
        #[track_caller]
        fn ast(self) -> AST {
          Var::from(self).ast()
        }
      }
      impl CmtLit for $t {
        #[track_caller]
        fn lit(self, t: &Type) -> Var {
          literal(self, t)
        }
        fn name(&self) -> String{
          format!("{}", self)
        }
      }
    )*
  };
}

make_from_primitive!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl From<bool> for Var {
  #[track_caller]
  fn from(value: bool) -> Self { literal(if value { 1 } else { 0 }, &Type::UInt(1)) }
}
impl CmtAST for bool {
  #[track_caller]
  fn ast(self) -> AST { Into::<Var>::into(self).ast() }
}

// TODO : imple form bigint / biguint
impl TryFrom<BigInt> for Var {
  type Error = String;

  #[track_caller]
  fn try_from(value: BigInt) -> Result<Self, Self::Error> {
    let lit = cmtir::RadixIntLit::from_bigint(value)?;
    let ty = ir::Type::UInt(lit.binary_len() as u32);
    Ok(Var::new(AST::lit(TypedLit::new(lit, ty), None)))
  }
}

/// Allow to be printed in [`SimPrint`]
pub trait ToPrintItem {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem;
}

impl ToPrintItem for Var {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem {
    let id = if let Some(id) = self.value_id() { id } else { crate::var!(self).value_id().unwrap() };
    PrintItem::Value(id)
  }
}

impl ToPrintItem for Vars {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem {
    assert!(self.0.len() == 1);
    let id = if let Some(id) = self.0[0].value_id() { id } else { crate::var!(self).value_id().unwrap() };
    PrintItem::Value(id)
  }
}

impl ToPrintItem for String {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { PrintItem::String(self.to_string()) }
}

impl ToPrintItem for str {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { PrintItem::String(self.to_string()) }
}

impl ToPrintItem for &str {
  fn to_print_item(&self, __cmt_gen: ContextItfc<impl Itfc>) -> PrintItem { PrintItem::String(self.to_string()) }
}

pub trait ToIOMacro {
  type Output;
  fn to_io_macro(self) -> Self::Output;
}

impl ToIOMacro for &Type {
  type Output = Type;

  fn to_io_macro(self) -> Self::Output { self.clone() }
}
impl ToIOMacro for Type {
  type Output = Type;

  fn to_io_macro(self) -> Self::Output { self }
}
