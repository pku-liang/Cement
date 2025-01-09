use crate::TimingPartialOrder;

use super::*;

#[derive(Debug, Clone)]
pub enum DepKind {
  Unknown,
  Directive(TConstraint),
  DefUse(TConstraint),
  Conflict,
  Invoke(InstRule),
}

impl ToString for DepKind {
  fn to_string(&self) -> String {
    match self {
      DepKind::Invoke(inst_rule) => format!("invoke {}", inst_rule.to_string()),
      DepKind::Directive(c) => format!("directive {}", c.to_string()),
      DepKind::DefUse(c) => format!("defuse {}", c.to_string()),
      DepKind::Conflict => "conflict".to_string(),
      DepKind::Unknown => "unknown".to_string(),
    }
  }
}

impl DepKind {
  pub fn constraint(&self) -> Option<TConstraint> {
    match self {
      DepKind::Directive(c) => Some(*c),
      DepKind::DefUse(c) => Some(*c),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum TDelta {
  LessEq(u32),
  Eq(u32),
}

impl ToString for TDelta {
  fn to_string(&self) -> String {
    match self {
      TDelta::LessEq(x) => format!(">=+{}", x),
      TDelta::Eq(x) => format!("=+{}", x),
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct TConstraint {
  pub is_start: bool,
  pub delta: TDelta,
}

impl ToString for TConstraint {
  fn to_string(&self) -> String {
    format!(
      "{} from {}",
      self.delta.to_string(),
      if self.is_start { "start" } else { "end" },
    )
  }
}

#[derive(Debug, Clone)]
pub struct Dep {
  pub kind: DepKind,
  pub pred_idx: Option<usize>,
}

impl ToString for Dep {
  fn to_string(&self) -> String {
    format!(
      "[{}] of {}-th",
      self.kind.to_string(),
      self.pred_idx.map_or("_".to_string(), |idx| idx.to_string())
    )
  }
}

pub trait DepModelTrait: Into<DepModel> {
  fn new(
    items: &[SchedItem],
    data: &VisitorData,
    pass: &mut MSchedPass,
  ) -> Self;
  fn report(&self, prefix_str: &str);
  // return the predecessors of the given op, and their min delta to the given
  // op
  fn get_preds(&self, idx: usize) -> Vec<Dep>;
}

#[derive(Debug, Clone)]
pub enum DepModel {
  Directive(DirectiveDepModel),
  DefUse(DefUseDepModel),
}

impl DepModelTrait for DepModel {
  fn new(
    _items: &[SchedItem],
    _data: &VisitorData,
    _pass: &mut MSchedPass,
  ) -> Self {
    unreachable!("DepModel is a variant, should not be instantiated directly")
  }

  fn report(&self, prefix_str: &str) {
    match self {
      DepModel::Directive(model) => model.report(prefix_str),
      DepModel::DefUse(model) => model.report(prefix_str),
    }
  }

  fn get_preds(&self, idx: usize) -> Vec<Dep> {
    match self {
      DepModel::Directive(model) => model.get_preds(idx),
      DepModel::DefUse(model) => model.get_preds(idx),
    }
  }
}

impl DepModel {
  pub fn name(&self) -> String {
    match self {
      DepModel::Directive(_) => "directive".to_string(),
      DepModel::DefUse(_) => "defuse".to_string(),
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum DirectiveSep {
  Delta(u32),
  None,
}

impl From<u32> for DirectiveSep {
  fn from(delta: u32) -> Self {
    DirectiveSep::Delta(delta)
  }
}
impl DirectiveSep {
  fn delta(&self) -> u32 {
    match self {
      DirectiveSep::Delta(delta) => *delta,
      DirectiveSep::None => 0,
    }
  }

  fn is_delta(&self) -> bool {
    matches!(self, DirectiveSep::Delta(_))
  }

  fn is_delta_and(&self, f: impl FnOnce(u32) -> bool) -> bool {
    matches!(self, DirectiveSep::Delta(x) if f(*x))
  }
}

#[derive(Debug, Clone)]
pub struct DirectiveDepModel {
  ranges: AutoVec<(Option<ir::TimingExpr>, Option<ir::TimingExpr>)>,
}

impl Into<DepModel> for DirectiveDepModel {
  fn into(self) -> DepModel {
    DepModel::Directive(self)
  }
}

impl DepModelTrait for DirectiveDepModel {
  fn new(
    items: &[SchedItem],
    _data: &VisitorData,
    _pass: &mut MSchedPass,
  ) -> Self {
    let mut ranges: AutoVec<(Option<ir::TimingExpr>, Option<ir::TimingExpr>)> =
      AutoVec::new();

    let mut opens: Vec<(usize, Option<ir::TimingExpr>)> = Vec::new();

    for (i, item) in items.iter().enumerate() {
      match item {
        SchedItem::Op(_) => {
          ranges.insert(i, (opens.last().unwrap().clone().1, None));
        }
        SchedItem::Tl(is_close, texpr) => {
          if *is_close && texpr.is_some() {
            let (idx, _texpr) = opens.pop().unwrap();
            for j in (idx..i).rev() {
              if ranges.has(j) {
                if ranges.get(j).1.is_some() {
                  break;
                }
                ranges.get_mut(j).1 = texpr.clone();
              }
            }
          } else if !is_close {
            opens.push((i, texpr.clone()));
          }
        }
      }
    }

    Self { ranges }
  }

  fn get_preds(&self, idx: usize) -> Vec<Dep> {
    let mut preds = Vec::new();
    // end-texpr is omitted, since we only use the start-texpr for
    // constraint now
    let (this_start_texpr, _) = self.ranges.get(idx).clone();

    if let Some(this_start_texpr) = this_start_texpr {
      for i in 0..idx {
        if self.ranges.has(i) {
          let (start_texpr, end_texpr) = self.ranges.get(i).clone();
          if let Some(start_texpr) = start_texpr {
            if let Some(delta) = this_start_texpr.sub(&start_texpr) {
              preds.push(Dep {
                kind: DepKind::Directive(TConstraint {
                  is_start: true,
                  delta: TDelta::LessEq(delta),
                }),
                pred_idx: Some(i),
              });
            }
          }
          if let Some(end_texpr) = end_texpr {
            if let Some(delta) = this_start_texpr.sub(&end_texpr) {
              preds.push(Dep {
                kind: DepKind::Directive(TConstraint {
                  is_start: false,
                  delta: TDelta::LessEq(delta),
                }),
                pred_idx: Some(i),
              });
            }
          }
        }
      }
    }

    preds
  }

  fn report(&self, prefix_str: &str) {
    let report_str = self
      .ranges
      .iter()
      .filter(|(_i, range)| range.0.is_some() || range.1.is_some())
      .map(|(i, range)| {
        format!("{}: [{}, {}]", i, range.0.ir_dump(), range.1.ir_dump())
      })
      .collect::<Vec<_>>()
      .join("\n\t");
    log::debug!("directive-dep for {}: \n\t{}", prefix_str, report_str);
  }
}

#[derive(Debug, Clone)]
pub struct DefUseDepModel {
  // value: (def_by, delta-from-start-cycle)
  pub value_def_by: SecondaryMap<ir::ValueId, (usize, Option<u32>)>,
  // value: (value_id, delta-from-start-cycle)
  pub use_of: AutoVec<Vec<(ir::ValueId, Option<u32>)>>,
}

impl Into<DepModel> for DefUseDepModel {
  fn into(self) -> DepModel {
    DepModel::DefUse(self)
  }
}

impl DepModelTrait for DefUseDepModel {
  fn new(
    items: &[SchedItem],
    data: &VisitorData,
    pass: &mut MSchedPass,
  ) -> Self {
    let mut value_def_by = SecondaryMap::new();
    let mut use_of = AutoVec::new();
    for (i, item) in items.iter().enumerate() {
      if let SchedItem::Op(op) = item {
        for (idx, output) in op.outputs().enumerate() {
          let delta = data.get_output_delta(
            op,
            idx,
            &pass.rule_sched_table,
            &pass.value_context_table,
          );
          value_def_by.insert(output, (i, delta));
        }
        let inputs = op
          .inputs()
          .enumerate()
          .map(|(input_idx, v)| {
            (
              v,
              data.get_input_delta(
                op,
                input_idx,
                &pass.rule_sched_table,
                &pass.value_context_table,
              ),
            )
          })
          .collect::<Vec<_>>();
        use_of.insert(i, inputs);
      }
    }

    Self {
      value_def_by,
      use_of,
    }
  }

  fn report(&self, prefix_str: &str) {
    let report_str = self
      .value_def_by
      .iter()
      .map(|(value, (def_by, delta))| {
        format!(
          "{}: {} (def-by: {}, delta: {:?})",
          value,
          delta.map_or("".to_string(), |d| format!("delta: {}", d)),
          def_by,
          delta
        )
      })
      .collect::<Vec<_>>()
      .join("\n\t");
    log::debug!("def-use-dep for {}: \n\t{}", prefix_str, report_str);
  }

  fn get_preds(&self, idx: usize) -> Vec<Dep> {
    let mut preds = Vec::new();
    let inputs = self.use_of.get(idx);
    for (input, input_delta) in inputs {
      if let Some((def_by, output_delta)) = self.value_def_by.get(*input) {
        if *def_by < idx {
          preds.push(Dep {
            kind: DepKind::DefUse(TConstraint {
              is_start: output_delta.is_some(),
              delta: TDelta::LessEq(match (output_delta, input_delta) {
                (Some(output_delta), Some(input_delta)) => {
                  *output_delta - *input_delta
                }
                (Some(output_delta), None) => *output_delta,
                (None, Some(_input_delta)) => 0,
                (None, None) => 0,
              }),
            }),
            pred_idx: Some(*def_by),
          });
        }
      }
    }
    preds
  }
}
