//! FIRRTL.
//!
//! For simplicity, we omitted some items such as `Info`, `StringLit`,
//! `ExtModule`, ..

// use crate::codegen::*;
// use crate::lir;
use core::panic;
use std::ops::Deref;

/// Indents every line in the string.
pub fn indent(str: String, indent: usize) -> String {
  let end_with_newline = str.ends_with('\n');
  let indented_str = str
    .lines()
    .map(|l| format!("{}{}", " ".repeat(indent), l))
    .collect::<Vec<_>>()
    .join("\n");
  if end_with_newline {
    format!("{}\n", indented_str)
  } else {
    indented_str
  }
}

const INDENT: usize = 2;

/// Logic value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicValue {
  /// Logic '0' or false condition
  False,
  /// Logic '1' or true condition
  True,
  /// Don't care or unknown value
  X,
  /// High impedance state (used for tri-state buffer)
  Z,
}

impl LogicValue {
  pub fn from_bool(b: bool) -> Self {
    if b {
      LogicValue::True
    } else {
      LogicValue::False
    }
  }
}

impl ToString for LogicValue {
  fn to_string(&self) -> String {
    match self {
      LogicValue::False => "0",
      LogicValue::True => "1",
      LogicValue::X => "x",
      LogicValue::Z => "z",
    }
    .to_string()
  }
}

/// Logic values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicValues(Vec<LogicValue>);

impl ToString for LogicValues {
  fn to_string(&self) -> String {
    self.0.iter().map(|b| b.to_string()).collect::<String>()
  }
}

impl Deref for LogicValues {
  type Target = [LogicValue];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl LogicValues {
  /// Creates new logic values.
  pub fn new(inner: Vec<LogicValue>) -> Self {
    Self(inner)
  }

  /// Inner logic values.
  pub fn into_inner(self) -> Vec<LogicValue> {
    self.0
  }
}

/// Direction of port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
  /// Input
  Input,

  /// Output
  Output,
}

impl ToString for Direction {
  fn to_string(&self) -> String {
    match self {
      Direction::Input => "input".to_string(),
      Direction::Output => "output".to_string(),
    }
  }
}

/// Primitive operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimOp {
  /// Addition
  Add,

  /// Subtraction
  Sub,

  /// Multiplication
  Mul,

  /// Division
  Div,

  /// Remainder
  Rem,

  /// Less Than
  Lt,

  /// Less Than Or Equal To
  Leq,

  /// Greater Than
  Gt,

  /// Greater Than Or Equal To
  Geq,

  /// Equal To
  Eq,

  /// Not Equal To
  Neq,

  /// Padding
  Pad,

  /// Static Shift Left
  Shl,

  /// Static Shift Right
  Shr,

  /// Dynamic Shift Left
  Dshl,

  /// Dynamic Shift Right
  Dshr,

  /// Bitwise Complement
  Not,

  /// Negate
  Neg,

  /// Bitwise And
  And,

  /// Bitwise Or
  Or,

  /// Bitwise Exclusive Or
  Xor,

  /// Bitwise And Reduce
  Andr,

  /// Bitwise Or Reduce
  Orr,

  /// Bitwise Exclusive Or Reduce
  Xorr,

  /// Concatenate
  Cat,

  /// Bit Extraction
  Bits,

  /// Head
  Head,

  /// Tail
  Tail,

  /// Cvt
  Cvt,

  /// AsUInt
  AsUInt,

  /// AsSInt
  AsSInt,
}

impl ToString for PrimOp {
  fn to_string(&self) -> String {
    match self {
      PrimOp::Add => "add",
      PrimOp::Sub => "sub",
      PrimOp::Mul => "mul",
      PrimOp::Div => "div",
      PrimOp::Rem => "rem",
      PrimOp::Lt => "lt",
      PrimOp::Leq => "leq",
      PrimOp::Gt => "gt",
      PrimOp::Geq => "geq",
      PrimOp::Eq => "eq",
      PrimOp::Neq => "neq",
      PrimOp::Pad => "pad",
      PrimOp::Shl => "shl",
      PrimOp::Shr => "shr",
      PrimOp::Dshl => "dshl",
      PrimOp::Dshr => "dshr",
      PrimOp::Not => "not",
      PrimOp::Neg => "neg",
      PrimOp::And => "and",
      PrimOp::Or => "or",
      PrimOp::Xor => "xor",
      PrimOp::Andr => "andr",
      PrimOp::Orr => "orr",
      PrimOp::Xorr => "xorr",
      PrimOp::Cat => "cat",
      PrimOp::Bits => "bits",
      PrimOp::Head => "head",
      PrimOp::Tail => "tail",
      PrimOp::Cvt => "cvt",
      PrimOp::AsUInt => "asUInt",
      PrimOp::AsSInt => "asSInt",
    }
    .to_string()
  }
}

impl PrimOp {
  /// Returns true if `self` is comparison operator.
  #[inline]
  pub fn is_comparison(self) -> bool {
    matches!(
      self,
      PrimOp::Lt
        | PrimOp::Leq
        | PrimOp::Gt
        | PrimOp::Geq
        | PrimOp::Eq
        | PrimOp::Neq
    )
  }

  /// Returns true if `self` is binary bitwise operator.
  #[inline]
  pub fn is_binary_bitwise(self) -> bool {
    matches!(self, PrimOp::And | PrimOp::Or | PrimOp::Xor)
  }

  /// Returns true if `self` is bitwise reduction operator.
  #[inline]
  pub fn is_bitwise_reduction(self) -> bool {
    matches!(self, PrimOp::Andr | PrimOp::Orr | PrimOp::Xorr)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
  Signed,
  Unsigned,
}

/// Expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
  /// Previously declared circuit component.
  Reference {
    /// placeholder for transform passes.
    prefix: bool,
    /// Name of the component
    name: String,
  },
  /// Sub-element of an expression with a bundle type.
  SubField {
    /// Input signal
    expr: Box<Expression>,
    /// Name of the field
    name: String,
  },
  /// Sub-element of an expression with a vector type.
  SubIndex {
    /// Input signal
    expr: Box<Expression>,
    /// Index of the element
    value: usize,
  },
  /// Sub-element of a vector-typed expression using a calculated index.
  SubAccess {
    /// Input signal
    expr: Box<Expression>,
    /// Index of the element
    index: Box<Expression>,
  },
  /// One of two input expressions depending on the value of an unsigned
  /// selection signal.
  Mux {
    /// Selection signal
    cond: Box<Expression>,
    /// Selected signal when `cond` is true
    tval: Box<Expression>,
    /// Selected signal when `cond` is false
    fval: Box<Expression>,
  },
  /// Literal.
  Literal {
    /// Signness
    signness: Sign,
    /// Value represented in binary format
    value: String,
    /// Width
    width: Option<usize>,
  },
  /// Primitive operation.
  DoPrim {
    /// Primitive operator
    op: PrimOp,
    /// Arguments
    args: Vec<Expression>,
    /// Constants
    consts: Vec<usize>,
  },
}

impl ToString for Expression {
  fn to_string(&self) -> String {
    match self {
      Expression::Reference { prefix, name } => {
        format!("{}{}", if *prefix { "_." } else { "" }, name.clone())
      }
      Expression::SubField { expr, name } => {
        format!("{}.{}", expr.to_string(), name)
      }
      Expression::SubIndex { expr, value } => {
        format!("{}[{}]", expr.to_string(), value)
      }
      Expression::SubAccess { expr, index } => {
        format!("{}[{}]", expr.to_string(), index.to_string())
      }
      Expression::Mux { cond, tval, fval } => {
        format!(
          "mux({}, {}, {})",
          cond.to_string(),
          tval.to_string(),
          fval.to_string()
        )
      }
      Expression::Literal {
        signness,
        value,
        width,
      } => format!(
        "{}{}({})",
        match signness {
          Sign::Signed => "SInt",
          Sign::Unsigned => "UInt",
        },
        match width {
          None => "".to_string(),
          Some(width) => format!("<{}>", width),
        },
        value
      ),
      Expression::DoPrim { op, args, consts } => {
        format!(
          "{}({})",
          op.to_string(),
          ::std::iter::empty()
            .chain(args.iter().map(|s| s.to_string()))
            .chain(consts.iter().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join(", ")
        )
      }
    }
  }
}

impl From<usize> for Expression {
  fn from(n: usize) -> Self {
    Expression::Literal {
      signness: Sign::Unsigned,
      value: n.to_string(),
      width: None,
    }
  }
}

#[allow(clippy::should_implement_trait)]
impl Expression {
  #[inline]
  pub fn ones(width: usize) -> Self {
    Expression::Literal {
      signness: Sign::Unsigned,
      // value is all 1's
      value: format!("0b{}", "1".repeat(width)),
      width: Some(width),
    }
  }

  #[inline]
  pub fn zeros(width: usize) -> Self {
    Expression::Literal {
      signness: Sign::Unsigned,
      // value is all 0's
      value: format!("0b{}", "0".repeat(width)),
      width: Some(width),
    }
  }

  /// Reference expression.
  #[inline]
  pub fn reference(name: String) -> Self {
    Expression::Reference {
      prefix: false,
      name,
    }
  }

  #[inline]
  pub fn reference_with_prefix(name: String) -> Self {
    Expression::Reference { prefix: true, name }
  }

  /// Returns true if `self` is reference.
  #[inline]
  pub fn is_reference(&self) -> bool {
    matches!(self, Expression::Reference { .. })
  }

  /// Clock expression.
  #[inline]
  pub fn clk() -> Self {
    Expression::reference("clk".to_string())
  }

  /// Reset expression.
  #[inline]
  pub fn rst() -> Self {
    Expression::reference("rst".to_string())
  }

  /// Subfield expression.
  #[inline]
  pub fn sub_field(expr: Expression, name: String) -> Self {
    Expression::SubField {
      expr: Box::new(expr),
      name,
    }
  }

  /// Subindex expression.
  #[inline]
  pub fn sub_index(expr: Expression, value: usize) -> Self {
    Expression::SubIndex {
      expr: Box::new(expr),
      value,
    }
  }

  /// Subaccess expression.
  #[inline]
  pub fn sub_access(expr: Expression, index: Expression) -> Self {
    Expression::SubAccess {
      expr: Box::new(expr),
      index: Box::new(index),
    }
  }

  /// Mux expression.
  #[inline]
  pub fn mux(cond: Expression, tval: Expression, fval: Expression) -> Self {
    Expression::Mux {
      cond: Box::new(cond),
      tval: Box::new(tval),
      fval: Box::new(fval),
    }
  }

  /// X expression.
  #[inline]
  pub fn x(width: usize) -> Self {
    Self::literal(
      Sign::Unsigned,
      false,
      LogicValues::new(vec![LogicValue::X; width]),
      Some(width),
    )
  }
  /// Literal expression.
  ///
  /// Value should be represented in binary format. (e.g. "01001")
  #[inline]
  pub fn literal(
    signness: Sign,
    is_neg: bool,
    value: LogicValues,
    width: Option<usize>,
  ) -> Self {
    Expression::Literal {
      signness,
      value: format!(
        "{}0b{}",
        if is_neg { "-" } else { "" },
        value.to_string()
      ),
      width,
    }
  }

  /// Primitive operation.
  #[inline]
  pub fn do_prim(
    op: PrimOp,
    args: Vec<Expression>,
    consts: Vec<usize>,
  ) -> Self {
    Expression::DoPrim { op, args, consts }
  }

  /// Add operation.
  ///
  /// Result width is `max(w_e1, w_e2) + 1`.
  #[inline]
  pub fn add(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Add, vec![e1, e2], Vec::new())
  }

  /// Subtract operation.
  ///
  /// Result width is `max(w_e1, w_e2) + 1`.
  #[inline]
  pub fn sub(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Sub, vec![e1, e2], Vec::new())
  }

  /// Multiply operation.
  ///
  /// Result width is `w_e1 + w_e2`.
  #[inline]
  pub fn mul(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Mul, vec![e1, e2], Vec::new())
  }

  /// Divide operation.
  ///
  /// Result width is `w_num`.
  #[inline]
  pub fn div(num: Self, den: Self) -> Self {
    Expression::do_prim(PrimOp::Div, vec![num, den], Vec::new())
  }

  /// Modulus operation.
  ///
  /// Result width is `min(w_num, w_den)`.
  #[inline]
  pub fn rem(num: Self, den: Self) -> Self {
    Expression::do_prim(PrimOp::Rem, vec![num, den], Vec::new())
  }

  /// Comparison operations. (lt, leq, gt, geq, eq, neq)
  ///
  /// Result width is `1`.
  #[inline]
  pub fn cmp(op: PrimOp, e1: Self, e2: Self) -> Self {
    assert!(op.is_comparison());
    Expression::do_prim(op, vec![e1, e2], Vec::new())
  }

  /// Padding operation.
  ///
  /// Result width is `max(w_e, n)`.
  #[inline]
  pub fn pad(e: Self, n: usize) -> Self {
    Expression::do_prim(PrimOp::Pad, vec![e], vec![n])
  }

  /// Shift left operation.
  ///
  /// Result width is `w_e + n`.
  #[inline]
  pub fn shl(e: Self, n: usize) -> Self {
    Expression::do_prim(PrimOp::Shl, vec![e], vec![n])
  }

  /// Shift right operation.
  ///
  /// Result width is `max(w_e - n, 1)`.
  #[inline]
  pub fn shr(e: Self, n: usize) -> Self {
    Expression::do_prim(PrimOp::Shr, vec![e], vec![n])
  }

  /// Dynamic shift left operation.
  ///
  /// Result width is `w_e1 + 2 ^ w_e2 - 1`.
  #[inline]
  pub fn dshl(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Dshl, vec![e1, e2], Vec::new())
  }

  /// Dynamic shift right operation.
  ///
  /// Result width is `w_e1`.
  #[inline]
  pub fn dshr(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Dshr, vec![e1, e2], Vec::new())
  }

  /// Bitwise complement operation.
  ///
  /// Result width is `w_e`.
  #[inline]
  pub fn not(e: Self) -> Self {
    Expression::do_prim(PrimOp::Not, vec![e], Vec::new())
  }

  /// Binary bitwise operations. (and, or, xor)
  ///
  /// Result width is `max(w_e1, w_e2)`.
  #[inline]
  pub fn binary_bitwise(op: PrimOp, e1: Self, e2: Self) -> Self {
    assert!(op.is_binary_bitwise());
    Expression::do_prim(op, vec![e1, e2], Vec::new())
  }

  #[inline]
  pub fn and(e1: Self, e2: Self) -> Self {
    Expression::binary_bitwise(PrimOp::And, e1, e2)
  }

  #[inline]
  pub fn or(e1: Self, e2: Self) -> Self {
    Expression::binary_bitwise(PrimOp::Or, e1, e2)
  }

  /// Bitwise reduction operations. (andr, orr, xorr)
  ///
  /// Result width is `1`.
  #[inline]
  pub fn bitwise_reduction(op: PrimOp, e: Self) -> Self {
    assert!(op.is_bitwise_reduction());
    Expression::do_prim(op, vec![e], Vec::new())
  }

  #[inline]
  pub fn and_reduce(e: Self) -> Self {
    Expression::bitwise_reduction(PrimOp::Andr, e)
  }

  #[inline]
  pub fn or_reduce(e: Self) -> Self {
    Expression::bitwise_reduction(PrimOp::Orr, e)
  }

  /// Concatenate operation.
  ///
  /// Result width is `w_e1 + w_e2`.
  #[inline]
  pub fn cat(e1: Self, e2: Self) -> Self {
    Expression::do_prim(PrimOp::Cat, vec![e1, e2], Vec::new())
  }

  /// Bit extraction operation.
  ///
  /// Result width is `hi - lo + 1`.
  #[inline]
  pub fn bits(e: Self, hi: usize, lo: usize) -> Self {
    Expression::do_prim(PrimOp::Bits, vec![e], vec![hi, lo])
  }

  /// Head.
  ///
  /// Result width is `n`.
  #[inline]
  pub fn head(e: Self, n: usize) -> Self {
    Expression::do_prim(PrimOp::Head, vec![e], vec![n])
  }

  /// Tail.
  ///
  /// Result width is `w_e - n`.
  #[inline]
  pub fn tail(e: Self, n: usize) -> Self {
    Expression::do_prim(PrimOp::Tail, vec![e], vec![n])
  }

  pub fn prefix_with(self, prefix: &str, sep: &str) -> Self {
    if prefix.is_empty() {
      return self;
    }
    match self {
      Expression::Reference { prefix: pf, name } => {
        if pf {
          Expression::Reference {
            prefix: true,
            name: format!("{}", name),
          }
        } else {
          Expression::Reference {
            prefix: false,
            name: format!("{}{}{}", prefix, sep, name),
          }
        }
      }
      Expression::SubField { expr, name } => Expression::SubField {
        expr: Box::new(expr.prefix_with(prefix, sep)),
        name,
      },
      Expression::SubIndex { expr, value } => Expression::SubIndex {
        expr: Box::new(expr.prefix_with(prefix, sep)),
        value,
      },
      Expression::SubAccess { expr, index } => Expression::SubAccess {
        expr: Box::new(expr.prefix_with(prefix, sep)),
        index: Box::new(index.prefix_with(prefix, sep)),
      },
      Expression::Mux { cond, tval, fval } => Expression::Mux {
        cond: Box::new(cond.prefix_with(prefix, sep)),
        tval: Box::new(tval.prefix_with(prefix, sep)),
        fval: Box::new(fval.prefix_with(prefix, sep)),
      },
      Expression::Literal {
        signness,
        value,
        width,
      } => Expression::Literal {
        signness,
        value,
        width,
      },
      Expression::DoPrim { op, args, consts } => Expression::DoPrim {
        op,
        args: args
          .into_iter()
          .map(|e| e.prefix_with(prefix, sep))
          .collect(),
        consts,
      },
    }
  }
}

/// Statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
  /// invalidate
  Invalidate {
    /// Expression
    expr: Expression,
  },
  /// Wire definition.
  DefWire {
    /// Prefix
    prefix: bool,
    /// Name of the wire
    name: String,
    /// Type of the signal
    tpe: Type,
  },
  /// Register definition.
  DefRegister {
    /// Name of the register
    name: String,
    /// Type of the signal
    tpe: Type,
    /// Clock signal
    clock: Expression,
    /// Reset signal
    reset: Expression,
    /// Initialization signal
    init: Expression,
  },
  /// Module instantiation.
  DefInstance {
    /// Name of the instance
    name: String,
    /// Name of the module
    module: String,
  },
  /// Intermediate value.
  DefNode {
    /// Name of the value
    name: String,
    /// Value
    value: Expression,
  },
  /// Conditional statement.
  Conditionally {
    /// Predicate signal
    pred: Expression,
    /// Then statement
    conseq: Box<Statement>,
    /// Else statement
    alt: Box<Statement>,
  },
  /// Block of statements.
  Block {
    /// Statements
    stmts: Vec<Statement>,
  },

  /// Physically wired connection between two circuit components.
  ///
  /// # Note
  ///
  /// Difference between `PartialConnect` is that `PartialConnect` enforces
  /// fewer restrictions on the types and widths of the circuit components it
  /// connects.
  ///
  /// For more details, see FIRRTL spec.
  Connect {
    /// L-value
    loc: Expression,
    /// R-value
    expr: Expression,
  },
  /// Indicate that a circuit component contains indeterminate value.
  IsInvalid {
    /// Expression
    expr: Expression,
  },
  /// Empty statement.
  EmptyStmt,
}

impl ToString for Statement {
  fn to_string(&self) -> String {
    match self {
      Statement::Invalidate { expr } => {
        format!("invalidate {}", expr.to_string())
      }
      Statement::DefWire { prefix, name, tpe } => {
        format!(
          "wire {}{} : {}",
          if *prefix { "__" } else { "" },
          name,
          tpe.to_string()
        )
      }
      Statement::DefRegister {
        name,
        tpe,
        clock,
        reset,
        init,
      } => {
        format!(
          "reg {} : {}, {} with :\n{}",
          name,
          tpe.to_string(),
          clock.to_string(),
          indent(
            format!("reset => ({}, {})", reset.to_string(), init.to_string()),
            INDENT
          )
        )
      }
      Statement::DefInstance { name, module } => {
        format!("inst {} of {}", name, module)
      }
      Statement::DefNode { name, value } => {
        format!("node {} = {}", name, value.to_string())
      }
      Statement::Conditionally { pred, conseq, alt } => {
        format!(
          "when {} :\n{}{}",
          pred.to_string(),
          indent(conseq.to_string(), INDENT),
          if matches!(**alt, Statement::EmptyStmt) {
            "".to_string()
          } else {
            format!("\nelse :\n{}", indent(alt.to_string(), INDENT))
          }
        )
      }
      Statement::Block { stmts } => {
        let res = stmts
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<_>>()
          .join("\n");

        if res.is_empty() {
          Statement::EmptyStmt.to_string()
        } else {
          res
        }
      }
      Statement::Connect { loc, expr } => {
        format!("connect {}, {}", loc.to_string(), expr.to_string())
      }
      Statement::IsInvalid { expr } => {
        format!("{} is invalid", expr.to_string())
      }
      Statement::EmptyStmt => "skip".to_string(),
    }
  }
}

impl Statement {
  /// Creates new invalidate statement.
  #[inline]
  pub fn invalidate(expr: Expression) -> Self {
    Statement::Invalidate { expr }
  }

  /// Creates new wire definition.
  #[inline]
  pub fn def_wire(name: String, tpe: Type, prefix: bool) -> Self {
    Statement::DefWire { prefix, name, tpe }
  }

  /// Creates new reg definition.
  #[inline]
  pub fn def_reg(name: String, tpe: Type, init: Expression) -> Self {
    Statement::DefRegister {
      name,
      tpe,
      clock: Expression::clk(),
      reset: Expression::rst(),
      init,
    }
  }

  /// Creates new module instantiation.
  #[inline]
  pub fn def_inst(name: String, module: String) -> Self {
    Statement::DefInstance { name, module }
  }

  /// Creates new node definition.
  #[inline]
  pub fn def_node(name: String, value: Expression) -> Self {
    Statement::DefNode { name, value }
  }

  /// Creates new block statement.
  #[inline]
  pub fn block(stmts: Vec<Statement>) -> Self {
    Statement::Block { stmts }
  }

  /// Creates new connect statement.
  #[inline]
  pub fn connect(loc: Expression, expr: Expression) -> Self {
    Statement::Connect { loc, expr }
  }

  /// When
  #[inline]
  pub fn when(
    pred: Expression,
    conseq: Statement,
    alt: Statement,
  ) -> Statement {
    Statement::Conditionally {
      pred,
      conseq: Box::new(conseq),
      alt: Box::new(alt),
    }
  }

  #[inline]
  pub fn push(&mut self, stmt: Statement) {
    self.extend(vec![stmt]);
  }

  #[inline]
  pub fn extend(&mut self, stmts: Vec<Statement>) {
    match self {
      Statement::Block { stmts: exsiting } => exsiting.extend(stmts),
      old @ _ => {
        let old_stmt = old.clone();
        let mut new_vec = vec![old_stmt];
        new_vec.extend(stmts);
        *old = Statement::block(new_vec);
      }
    }
  }

  pub fn prefix_with(self, prefix: &str, sep: &str) -> Self {
    match self {
      Statement::Invalidate { expr } => Statement::Invalidate {
        expr: expr.prefix_with(prefix, sep),
      },
      Statement::DefWire {
        prefix: pf,
        name,
        tpe,
      } => {
        if pf {
          Statement::DefWire {
            name,
            tpe,
            prefix: pf,
          }
        } else {
          Statement::DefWire {
            prefix: pf,
            name: format!("{}{}{}", prefix, sep, name),
            tpe,
          }
        }
      }
      Statement::DefRegister {
        name,
        tpe,
        clock,
        reset,
        init,
      } => Statement::DefRegister {
        name: format!("{}_{}", prefix, name),
        tpe,
        clock,
        reset,
        init,
      },
      Statement::DefInstance { name, module } => Statement::DefInstance {
        name: format!("{}_{}", prefix, name),
        module,
      },
      Statement::DefNode { name, value } => Statement::DefNode {
        name: format!("{}_{}", prefix, name),
        value,
      },
      Statement::Conditionally { pred, conseq, alt } => {
        Statement::Conditionally {
          pred,
          conseq: Box::new(conseq.prefix_with(prefix, sep)),
          alt: Box::new(alt.prefix_with(prefix, sep)),
        }
      }
      Statement::Block { stmts } => Statement::Block {
        stmts: stmts
          .into_iter()
          .map(|stmt| stmt.prefix_with(prefix, sep))
          .collect(),
      },
      Statement::Connect { loc, expr } => Statement::Connect {
        loc: loc.prefix_with(prefix, sep),
        expr: expr.prefix_with(prefix, sep),
      },
      Statement::IsInvalid { expr } => Statement::IsInvalid {
        expr: expr.prefix_with(prefix, sep),
      },
      Statement::EmptyStmt => Statement::EmptyStmt,
    }
  }
}

impl From<Vec<Statement>> for Statement {
  fn from(stmts: Vec<Statement>) -> Self {
    Statement::block(stmts)
  }
}

/// Type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
  /// Reset type.
  ResetType,
  /// Clock type.
  ClockType,
  /// Unsigned integer type.
  UIntType(usize),

  /// Signed integer type.
  SIntType(usize),

  /// Vector type
  VectorType { elem_ty: Box<Type>, size: usize },

  /// Bundle type.
  BundleType { elements: Vec<(Type, String, bool)> },
}

impl ToString for Type {
  fn to_string(&self) -> String {
    match self {
      Type::ResetType => "UInt<1>".to_string(),
      Type::ClockType => "Clock".to_string(),
      Type::UIntType(width) => format!("UInt<{}>", width),
      Type::SIntType(width) => format!("SInt<{}>", width),
      Type::VectorType { elem_ty, size } => {
        format!("{}[{}]", elem_ty.to_string(), size)
      }
      Type::BundleType { elements } => {
        let elems = elements
          .iter()
          .map(|(ty, name, flip)| {
            format!(
              "{}{} : {}",
              if *flip { "flip " } else { "" },
              name,
              ty.to_string()
            )
          })
          .collect::<Vec<_>>()
          .join(", ");
        format!("{{ {} }}", elems)
      }
    }
  }
}

impl Type {
  /// Creates new clock type.
  #[inline]
  pub fn clock() -> Self {
    Type::ClockType
  }

  /// Creates new unsigned integer type.
  #[inline]
  pub fn uint(width: usize) -> Self {
    Type::UIntType(width)
  }

  /// Creates new vector type.
  #[inline]
  pub fn vector(elem_ty: Type, size: usize) -> Self {
    Type::VectorType {
      elem_ty: Box::new(elem_ty),
      size,
    }
  }
}

/// Port of module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
  /// Name of the port
  pub name: String,
  /// Direction of the port
  pub direction: Direction,
  /// Type of the port
  pub tpe: Type,
}

impl Port {
  pub fn new(name: String, tpe: Type, direction: Direction) -> Self {
    Self {
      name,
      tpe,
      direction,
    }
  }
}

impl ToString for Port {
  fn to_string(&self) -> String {
    format!(
      "{} {} : {}",
      self.direction.to_string(),
      self.name,
      self.tpe.to_string(),
    )
  }
}

impl Port {
  /// Creates new input port.
  pub fn input(name: String, tpe: Type) -> Self {
    Port {
      name,
      direction: Direction::Input,
      tpe,
    }
  }

  /// Creates new output port.
  pub fn output(name: String, tpe: Type) -> Self {
    Port {
      name,
      direction: Direction::Output,
      tpe,
    }
  }
}

/// An instantiable hardware block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
  /// Name of the module
  pub name: String,
  /// Ports of the module
  pub ports: Vec<Port>,
  /// Defs
  pub defs: Vec<Statement>,
  /// Body of the module
  pub body: Statement,
}

impl Module {
  pub fn new(name: String) -> Self {
    Self {
      name,
      ports: Vec::new(),
      defs: Vec::new(),
      body: Statement::block(Vec::new()),
    }
  }
}

impl ToString for Module {
  fn to_string(&self) -> String {
    format!(
      "public module {} :\n{}\n\n{}\n{}",
      self.name,
      indent(
        self
          .ports
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<_>>()
          .join("\n"),
        INDENT
      ),
      indent(
        self
          .defs
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<_>>()
          .join("\n"),
        INDENT
      ),
      indent(self.body.to_string(), INDENT)
    )
  }
}

/// Circuit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circuit {
  /// Inner modules
  pub modules: Vec<Module>,
  /// Name of the circuit
  pub main: Option<String>,
}

impl Circuit {
  pub fn new() -> Self {
    Self {
      modules: Vec::new(),
      main: None,
    }
  }
}

impl ToString for Circuit {
  fn to_string(&self) -> String {
    format!(
      "FIRRTL version 4.0.0\ncircuit {} :\n{}\n",
      {
        if let Some(main) = &self.main {
          main.to_string()
        } else {
          panic!("main module is not specified")
        }
      },
      self
        .modules
        .iter()
        .map(|s| s.to_string())
        .map(|s| indent(s, INDENT))
        .collect::<Vec<_>>()
        .join("\n\n")
    )
  }
}
