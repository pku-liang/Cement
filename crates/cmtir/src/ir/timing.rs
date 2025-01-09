//! Timing expressions and partial order.

use std::{
  cmp::max,
  collections::{HashMap, HashSet, VecDeque},
};

use utils::{UFOrd, WeightedUnionFind};

use super::*;

#[derive(Debug, Clone, Hash)]
pub struct TimingExpr {
  pub var: Option<String>,
  pub const_: u32,
}

impl Default for TimingExpr {
  fn default() -> Self {
    TimingExpr {
      var: None,
      const_: 0,
    }
  }
}

impl TimingExpr {
  pub fn as_u32(&self) -> Option<u32> {
    if self.var.is_none() {
      Some(self.const_)
    } else {
      None
    }
  }

  pub fn var(name: impl AsRef<str>) -> Self {
    TimingExpr {
      var: Some(name.as_ref().to_string()),
      const_: 0,
    }
  }

  pub fn const_(value: u32) -> Self {
    TimingExpr {
      var: None,
      const_: value,
    }
  }

  pub fn var_only(self) -> Self {
    TimingExpr {
      var: self.var,
      const_: 0,
    }
  }

  pub fn add(self, rhs: TimingExpr) -> Self {
    if self.var.is_some() && rhs.var.is_some() {
      // we don't support adding two variables
      panic!("Unsupported operation: adding two variables");
    }
    TimingExpr {
      var: self.var.or(rhs.var),
      const_: self.const_ + rhs.const_,
    }
  }

  pub fn add_const(&self, delta: u32) -> Self {
    self.clone().add(TimingExpr::const_(delta))
  }

  pub fn eq(&self, other: &TimingExpr) -> bool {
    self.var == other.var && self.const_ == other.const_
  }

  pub fn ne(&self, other: &TimingExpr) -> bool {
    !self.eq(other)
  }

  pub fn has_same_var(&self, other: &TimingExpr) -> bool {
    self.var == other.var
  }

  // Some(true/false) if comparable, None if not
  pub fn le(&self, other: &TimingExpr) -> Option<bool> {
    if !self.has_same_var(other) {
      None
    } else {
      Some(self.const_ <= other.const_)
    }
  }

  // Some(true/false) if comparable, None if not
  pub fn lt(&self, other: &TimingExpr) -> Option<bool> {
    if !self.has_same_var(other) {
      None
    } else {
      Some(self.const_ < other.const_)
    }
  }

  pub fn max_or_else(
    &self,
    other: TimingExpr,
    f: impl FnOnce() -> TimingExpr,
  ) -> TimingExpr {
    if let Some(b) = self.le(&other) {
      if b {
        other
      } else {
        self.clone()
      }
    } else {
      f()
    }
  }

  pub fn sub(&self, other: &TimingExpr) -> Option<u32> {
    if self.has_same_var(other) {
      if self.const_ >= other.const_ {
        Some(self.const_ - other.const_)
      } else {
        None
      }
    } else {
      None
    }
  }

  pub fn is_static_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> bool {
    let lhs = tpo.canonicalize(self);
    let rhs = tpo.canonicalize(other);

    lhs.has_same_var(&rhs)
  }

  pub fn is_comparable_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> bool {
    !tpo.compare(self, other).unknown
  }

  pub fn sub_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> Option<u32> {
    if self.has_same_var(other) {
      if self.const_ >= other.const_ {
        Some(self.const_ - other.const_)
      } else {
        None
      }
    } else {
      tpo.sub_texpr(self, other)
    }
  }

  pub fn lt_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> Option<bool> {
    let cmp = tpo.compare(self, other);
    if cmp.unknown {
      None
    } else {
      Some(cmp.lt)
    }
  }

  pub fn ne_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> Option<bool> {
    let cmp = tpo.compare(self, other);
    if cmp.unknown {
      None
    } else {
      Some(!cmp.eq)
    }
  }

  pub fn le_with_tpo(
    &self,
    other: &TimingExpr,
    tpo: &mut TimingPartialOrder,
  ) -> Option<bool> {
    let cmp = tpo.compare(self, other);
    if cmp.unknown {
      None
    } else {
      Some(cmp.le)
    }
  }
}

impl PartialEq for TimingExpr {
  fn eq(&self, other: &Self) -> bool {
    self.var == other.var && self.const_ == other.const_
  }
}

impl Eq for TimingExpr {}

impl ToString for TimingExpr {
  fn to_string(&self) -> String {
    let mut s = String::new();
    if let Some(var) = &self.var {
      s.push_str(var);
    }
    if s.is_empty() {
      s.push_str(&self.const_.to_string());
    } else if self.const_ != 0 {
      if !s.is_empty() {
        s.push('+');
      }
      s.push_str(&self.const_.to_string());
    }
    s
  }
}

impl Parse for TimingExpr {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    let mut expr = TimingExpr::default();
    // parse [var + ]? const
    if parser.peek_fn(|_, t| t == Token::Keyword) {
      // var
      let str = parser.expect_any(&[Token::Keyword])?;
      expr.var = Some(str.to_string());
      if parser.peek_fn(|s, t| t == Token::Punct && s == "+") {
        // var + const
        let _ = parser.expect_str(Token::Punct, "+")?;
        expr.const_ = parser.parse()?;
      }
    } else if parser.peek_fn(|_, t| t == Token::Number) {
      // const
      expr.const_ = parser.parse()?;
    } else {
      return Err(format!(
        "Unexpected token: {:?}, here we need \"[var + ]? const\"",
        parser.peek()
      ));
    }
    Ok(expr)
  }
}

impl Print for TimingExpr {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    write!(p, "{}", self.to_string())
  }
}

// parse print! macro for TimingExpr, this macro can be accessed in other crates
#[macro_export]
macro_rules! texpr {
  ($($tt:tt)*) => {
    {
      let str = format!($($tt)*);
      TimingExpr::parse(&mut Parser::new(&str)).unwrap()
    }
  };
}

fn var_lt(lhs: &Option<String>, rhs: &Option<String>) -> bool {
  match (lhs, rhs) {
    (None, None) => false,
    (None, Some(_)) => true,
    (Some(_), None) => false,
    (Some(lhs), Some(rhs)) => lhs < rhs,
  }
}

impl UFOrd for TimingExpr {
  fn uf_lt(&self, other: &Self) -> bool {
    if self.const_ != other.const_ {
      self.const_ < other.const_
    } else {
      var_lt(&self.var, &other.var)
    }
  }
}

// A partial order on TimingExprs, it should include:
// - equality: union-find set to represent the equivalent (or
//   constant-delta-equal) classes of timing exprs;
//   - Property: every root of the uf is a var-only texpr
// - later-than relation: a HashMap from TimingExpr to a list of TimingExprs
//   that is earlier than it: (expr2 >= expr1 + delta)
#[derive(Debug, Clone)]
pub struct TimingPartialOrder {
  pub uf: WeightedUnionFind<TimingExpr>,
  pub later_than: HashMap<TimingExpr, HashMap<TimingExpr, u32>>,
  pub earilier_than: HashMap<TimingExpr, HashMap<TimingExpr, u32>>,
}

impl Default for TimingPartialOrder {
  fn default() -> Self {
    Self {
      uf: WeightedUnionFind::new(),
      later_than: HashMap::new(),
      earilier_than: HashMap::new(),
    }
  }
}

impl TimingPartialOrder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn roots(&self) -> Vec<TimingExpr> {
    self.uf.roots()
  }

  pub fn dump(&mut self) -> String {
    let mut s = String::new();
    s.push_str(&format!("uf:\n{}", self.uf.dump()));

    s.push_str("later than:\n");
    for (expr, succs) in self.later_than.iter() {
      for (succ, delta) in succs.iter() {
        s.push_str(&format!(
          "{} > {} + {}\n",
          expr.to_string(),
          succ.to_string(),
          delta
        ));
      }
    }
    s
  }

  // topological order of timing exprs (root only)
  pub fn order(&self) -> Vec<TimingExpr> {
    let roots = self.roots();
    let mut root_set: HashSet<TimingExpr> =
      HashSet::from_iter(roots.clone().into_iter());

    let mut later_than: HashMap<TimingExpr, HashSet<TimingExpr>> = self
      .later_than
      .clone()
      .into_iter()
      .map(|(k, v)| (k, v.into_iter().map(|e| e.0).collect()))
      .collect();

    // reverse of later_than
    let mut earilier_than: HashMap<TimingExpr, HashSet<TimingExpr>> = self
      .earilier_than
      .clone()
      .into_iter()
      .map(|(k, v)| (k, v.into_iter().map(|e| e.0).collect()))
      .collect();

    let mut order: Vec<TimingExpr> = Vec::new();

    let mut fifo: VecDeque<TimingExpr> = VecDeque::new();

    for root in roots.iter() {
      if !self.later_than.contains_key(root) {
        fifo.push_back(root.clone());
        root_set.remove(root);
      }
    }

    while let Some(expr) = fifo.pop_front() {
      order.push(expr.clone());
      if let Some(succs) = earilier_than.remove(&expr) {
        for succ in succs.iter() {
          if let Some(preds) = later_than.get_mut(succ) {
            assert!(preds.contains(&expr));
            preds.retain(|e| e != &expr);
            if preds.is_empty() {
              fifo.push_back(succ.clone());
            }
          }
        }
      }
    }

    if order.len() != self.roots().len() {
      panic!("Timing partial order is not a DAG");
    }

    order
  }

  pub fn canonicalize(&mut self, expr: &TimingExpr) -> TimingExpr {
    if !self.uf.exists(&expr) {
      // assert!(
      //   expr.var.is_none(),
      //   "only const timing expr can be used without adding to tpo"
      // );
      return self.add_texpr(expr.clone());
    }
    let (root, weight) = self.uf.find(expr.clone());
    root.add_const(weight)
  }

  pub fn sub_texpr(
    &mut self,
    lhs: &TimingExpr,
    rhs: &TimingExpr,
  ) -> Option<u32> {
    let lhs_canonicalized = self.canonicalize(lhs);
    let rhs_canonicalized = self.canonicalize(rhs);

    if lhs_canonicalized.has_same_var(&rhs_canonicalized) {
      Some(lhs_canonicalized.const_ - rhs_canonicalized.const_)
    } else {
      None
    }
  }

  fn add_texpr_to_uf(&mut self, expr: TimingExpr) -> TimingExpr {
    let (expr, weight) = self.uf.add_elem(expr);
    expr.add_const(weight)
  }

  pub fn add_texpr(&mut self, expr: TimingExpr) -> TimingExpr {
    let expr = self.add_texpr_to_uf(expr);
    if expr.const_ != 0 {
      let var_only_expr = self.add_texpr_to_uf(expr.clone().var_only());
      self.uf.union(var_only_expr, expr.clone(), expr.const_);
    }
    expr
  }

  // update the partial order when a root changes
  // old_root + delta = new_root
  fn update_po(
    &mut self,
    old_root: TimingExpr,
    new_root: TimingExpr,
    root_delta: i32,
  ) {
    let mut new_later_than: HashMap<TimingExpr, u32> = HashMap::new();
    if let Some(old_later_than) = self.later_than.remove(&old_root) {
      for (expr, delta) in old_later_than {
        assert!(delta as i32 + root_delta >= 0);
        new_later_than.insert(expr.clone(), (delta as i32 + root_delta) as u32);
        self.earilier_than.get_mut(&expr).unwrap().remove(&old_root);
        self
          .earilier_than
          .get_mut(&expr)
          .unwrap()
          .insert(new_root.clone(), (delta as i32 + root_delta) as u32);
      }
    }
    self.later_than.insert(new_root.clone(), new_later_than);

    if let Some(earilier_than) = self.earilier_than.remove(&old_root) {
      for (expr, delta) in earilier_than {
        self.later_than.get_mut(&expr).unwrap().remove(&old_root);
        self
          .later_than
          .get_mut(&expr)
          .unwrap()
          .insert(new_root.clone(), (delta as i32 + root_delta) as u32);
      }
    }
  }

  fn union_and_update(
    &mut self,
    lhs: TimingExpr,
    rhs: TimingExpr,
  ) -> (TimingExpr, u32) {
    let (old_lhs_root, old_lhs_weight) = self.uf.find(lhs.clone());
    let (old_rhs_root, old_rhs_weight) = self.uf.find(rhs.clone());

    let (expr, weight) = self.uf.union(lhs, rhs, 0);

    if old_lhs_root != expr {
      // old_lhs_root + old_lhs_weight == expr + weight
      self.update_po(
        old_lhs_root,
        expr.clone(),
        old_lhs_weight as i32 - weight as i32,
      );
    }

    if old_rhs_root != expr {
      // old_rhs_root + old_rhs_weight == expr + weight
      self.update_po(
        old_rhs_root,
        expr.clone(),
        old_rhs_weight as i32 - weight as i32,
      );
    }

    (expr, weight)
  }

  pub fn union_texpr(
    &mut self,
    lhs: TimingExpr,
    rhs: TimingExpr,
  ) -> TimingExpr {
    let (expr, weight) = self.union_and_update(lhs, rhs);
    let final_expr = expr.add_const(weight);

    final_expr
  }

  fn add_po_relation(&mut self, lhs: TimingExpr, rhs: TimingExpr, delta: u32) {
    self
      .later_than
      .entry(rhs.clone())
      .or_default()
      .entry(lhs.clone())
      .and_modify(|d| *d = max(*d, delta))
      .or_insert(delta);
    self
      .earilier_than
      .entry(lhs.clone())
      .or_default()
      .entry(rhs.clone())
      .and_modify(|d| *d = max(*d, delta))
      .or_insert(delta);
  }

  pub fn po_texpr(
    &mut self,
    lhs: TimingExpr,
    rhs: TimingExpr,
  ) -> (TimingExpr, TimingExpr, u32) {
    let lhs = self.canonicalize(&lhs);
    let rhs = self.canonicalize(&rhs);

    assert!(lhs.const_ >= rhs.const_);
    let delta = lhs.const_ - rhs.const_;
    self.add_po_relation(lhs.clone().var_only(), rhs.clone().var_only(), delta);

    (lhs.var_only(), rhs.var_only(), delta)
  }

  // TODO: add transitive property
  pub fn query_po(
    &mut self,
    lhs: &TimingExpr,
    rhs: &TimingExpr,
  ) -> Option<u32> {
    let lhs = self.canonicalize(lhs);
    let rhs = self.canonicalize(rhs);

    // log::debug!("query po: {} <= {} + ?", lhs.to_string(), rhs.to_string());

    self
      .later_than
      .get(&(rhs.clone().var_only()))
      .and_then(|later_than| {
        later_than.get(&lhs.clone().var_only()).and_then(|d| {
          if *d + rhs.const_ >= lhs.const_ {
            Some(*d + rhs.const_ - lhs.const_)
          } else {
            None
          }
        })
      })
  }
}
#[derive(Debug)]
pub struct TpoCompare {
  pub unknown: bool,
  pub eq: bool,
  pub lt: bool,
  pub le: bool,
}

impl TimingPartialOrder {
  pub fn compare(&mut self, lhs: &TimingExpr, rhs: &TimingExpr) -> TpoCompare {
    let lhs = self.canonicalize(lhs);
    let rhs = self.canonicalize(rhs);

    if lhs.has_same_var(&rhs) {
      TpoCompare {
        unknown: false,
        eq: lhs.const_ == rhs.const_,
        lt: lhs.const_ < rhs.const_,
        le: lhs.const_ <= rhs.const_,
      }
    } else {
      if let Some(delta) = self.query_po(&lhs, &rhs) {
        // lhs + delta <= rhs
        TpoCompare {
          unknown: false,
          eq: false,
          lt: delta > 0,
          le: true,
        }
      } else if let Some(_delta) = self.query_po(&rhs, &lhs) {
        // rhs + delta <= lhs
        TpoCompare {
          unknown: false,
          eq: false,
          lt: false,
          le: false,
        }
      } else {
        TpoCompare {
          unknown: true,
          eq: false,
          lt: false,
          le: false,
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test_create_texpr() {
    assert_eq!(texpr!("x"), TimingExpr::var("x"));
    assert_eq!(texpr!("1"), TimingExpr::const_(1));
    assert_eq!(
      texpr!("x+1"),
      TimingExpr::var("x").add(TimingExpr::const_(1))
    );
  }

  #[test]
  fn test_tpo_uf() {
    let mut tpo = TimingPartialOrder::new();
    let x = tpo.add_texpr(texpr!("x"));
    let y = tpo.add_texpr(texpr!("y"));
    let x1 = tpo.add_texpr(texpr!("x+1"));
    let y2 = tpo.add_texpr(texpr!("y+2"));
    assert_eq!(tpo.canonicalize(&x1), x.clone().add_const(1));
    assert_eq!(tpo.canonicalize(&y2), y.clone().add_const(2));

    println!("{}", tpo.uf.dump());

    tpo.union_texpr(x.clone(), y2.clone());
    println!("{}", tpo.uf.dump());

    assert_eq!(tpo.canonicalize(&x1), y.clone().add_const(3));
  }

  #[test]
  fn test_tpo_po() {
    let mut tpo = TimingPartialOrder::new();
    let x = tpo.add_texpr(texpr!("x"));
    let _y = tpo.add_texpr(texpr!("y"));
    let x1 = tpo.add_texpr(texpr!("x+1"));
    let y2 = tpo.add_texpr(texpr!("y+2"));
    let z = tpo.add_texpr(texpr!("z"));
    let z1 = tpo.add_texpr(texpr!("z+1"));

    tpo.union_texpr(x.clone(), y2.clone());

    tpo.po_texpr(y2.clone(), z1.clone());

    println!("{}", tpo.dump());

    tpo.po_texpr(x1.clone(), z.clone());

    println!("{}", tpo.dump());

    assert_eq!(tpo.query_po(&x, &z), Some(1));
  }
}

#[derive(Debug, Clone, OpIO)]
pub struct TimingIntv {
  pub start: Option<TimingExpr>,
  pub end: Option<TimingExpr>,
}

impl Parse for TimingIntv {
  fn parse(parser: &mut Parser) -> Result<Self, String> {
    if parser.peek_fn(|s, _t| s == "_") {
      parser.next()?;
      return Ok(TimingIntv::none());
    }
    let _ = parser.expect_str(Token::LParen, "(")?;
    let (start, end) = if parser.peek_fn(|s, _| s == ",") {
      let _ = parser.expect_str(Token::Punct, ",")?;
      let end = if parser.peek_fn(|s, _| s == ")") {
        None
      } else {
        Some(parser.parse()?)
      };
      (None, end)
    } else if parser.peek_fn(|s, _| s == ")") {
      (None, None)
    } else {
      let start = parser.parse()?;
      let _ = parser.expect_str(Token::Punct, ",")?;
      if parser.peek_fn(|s, _| s == ")") {
        (Some(start), None)
      } else {
        let end = parser.parse()?;
        (Some(start), Some(end))
      }
    };
    let _ = parser.expect_str(Token::RParen, ")")?;
    Ok(TimingIntv { start, end })
  }
}

impl Print for TimingIntv {
  fn print<'p>(&'p self, p: &mut Printer<'p>) {
    match (self.start.as_ref(), self.end.as_ref()) {
      (None, None) => write!(p, "_"),
      (Some(start), None) => write!(p, "({},)", start.to_string()),
      (None, Some(end)) => write!(p, "(,{})", end.to_string()),
      (Some(start), Some(end)) => {
        write!(p, "({},{})", start.to_string(), end.to_string())
      }
    }
  }
}

impl TimingIntv {
  pub fn none() -> Self {
    TimingIntv {
      start: None,
      end: None,
    }
  }

  pub fn both(start: TimingExpr, end: TimingExpr) -> Self {
    TimingIntv {
      start: Some(start),
      end: Some(end),
    }
  }
}
