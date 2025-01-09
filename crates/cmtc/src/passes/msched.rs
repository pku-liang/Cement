use std::{
  arch::x86_64::_SIDD_LEAST_SIGNIFICANT, fmt::Debug, path::Path,
  thread::current,
};

use indexmap::IndexMap;

use crate::{TimingExpr, TimingPartialOrder};

use super::*;

#[derive(Clone, Default)]
pub struct RuleSchedStat {
  pub is_combinational: bool,
  // Some if static, None if dynamic
  pub num_cycles: Option<u32>,
  // if the guard op is always true
  pub guard_always_true: bool,
}

impl ToString for RuleSchedStat {
  fn to_string(&self) -> String {
    format!(
      "is_comb? {};\tnum_cycle={:?};\tguard_always_true={}",
      self.is_combinational, self.num_cycles, self.guard_always_true
    )
  }
}

#[derive(Clone)]
pub struct IndexedOp {
  pub idx: Option<usize>,
  pub op: ir::Op,
}

#[derive(Clone, Default)]
pub struct OpsPerStep {
  pub time_expr: ir::TimingExpr,
  pub ops: Vec<IndexedOp>,
}

impl OpsPerStep {
  pub fn new(time_expr: ir::TimingExpr) -> Self {
    Self {
      time_expr,
      ops: Vec::new(),
    }
  }

  pub fn into_inner(self) -> (ir::TimingExpr, Vec<IndexedOp>) {
    (self.time_expr, self.ops)
  }

  pub fn extend(&mut self, ops: impl IntoIterator<Item = IndexedOp>) {
    self.ops.extend(ops);
  }
}

#[derive(Clone)]
pub struct RuleSched {
  pub stat: RuleSchedStat,
  pub hy_steps: IndexMap<ir::TimingExpr, OpsPerStep>,
  pub timing_po: TimingPartialOrder,
  pub index_to_deps: AutoVec<(ir::TimingExpr, Vec<Dep>)>,
  pub texpr_to_deps: IndexMap<ir::TimingExpr, Vec<Dep>>,
  pub end_cycle: AutoVec<TimingExpr>,
  pub exit_cycle: TimingExpr,
}

impl Debug for RuleSched {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.stat.to_string())
  }
}

impl Default for RuleSched {
  fn default() -> Self {
    Self {
      stat: RuleSchedStat {
        is_combinational: true,
        num_cycles: Some(0),
        guard_always_true: true,
      },
      hy_steps: IndexMap::new(),
      timing_po: TimingPartialOrder::default(),
      index_to_deps: AutoVec::new(),
      texpr_to_deps: IndexMap::new(),
      end_cycle: AutoVec::new(),
      exit_cycle: TimingExpr::const_(0),
    }
  }
}

impl RuleSched {
  pub fn is_combinational(&self) -> bool {
    self.stat.is_combinational
  }

  // NOTE: this can not be used when MSchedPass during any visit_rule_impl,
  // since it uses module.rule_by_name
  pub fn dump(&self, rule_name: &str, data: &VisitorData) -> String {
    let mut string = String::new();
    string.push_str(&format!("{}: --- \n", rule_name));
    string.push_str(&format!("{}\n", self.stat.to_string()));

    for (texpr, ops) in self.hy_steps.iter() {
      string.push_str(&format!("@{}:\n", texpr.to_string()));
      for op in ops.ops.iter() {
        string.push_str(&format!(
          "\t[{}-th]{}\n",
          op.idx
            .map(|idx| format!("{}", idx))
            .unwrap_or("".to_string()),
          op.op.ir_dump_with(&data.module.values)
        ));
      }
    }

    for (idx, (texpr, deps)) in self.index_to_deps.iter() {
      string.push_str(&format!("{}-th op@{}:\n", idx, texpr.to_string()));
      for dep in deps.iter() {
        string.push_str(&format!("\t{}\n", dep.to_string()));
      }
    }

    for (texpr, deps) in self.texpr_to_deps.iter() {
      string.push_str(&format!("texpr {}:\n", texpr.to_string()));
      for dep in deps.iter() {
        string.push_str(&format!("\t{}\n", dep.to_string()));
      }
    }

    string
  }
}

pub type RuleSchedTable = HashMap<RuleId, RuleSched>;
pub type ValueContextTable = HashMap<RuleId, ValueContext>;

pub struct MSchedPass {
  pub rule_sched_table: RuleSchedTable,
  pub value_context_table: ValueContextTable,
}

impl Visitor for MSchedPass {
  fn name() -> &'static str {
    "msched"
  }

  /// tb module should have its own msched pass
  fn skip_tb() -> bool {
    true
  }

  fn dump_results(&self, data: &mut VisitorData) {
    for (rule_path, rule_sched_info) in self.rule_sched_table.iter() {
      if rule_path.module_name == data.module.name {
        log::debug!(
          "\n{}: {}",
          rule_path.to_string(),
          rule_sched_info.dump(&rule_path.rule_name, data)
        );
      }
    }
  }

  fn prepare_visit_module_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    self.infer_rule_relations(data)?;
    Ok(())
  }

  fn get_rules_in_order(&mut self, data: &mut VisitorData) -> Vec<ir::Rule> {
    // NOTE: this will change the order of rules in the module, which is not
    // what we want. Thereby, we use before_visit_rules and after_visit_rules to
    // backup and restore the rules.
    self.get_rules_in_topo_order(data)
  }

  // the below two functions are used to backup and restore rules
  // because msched should not change the order of rules in the module
  fn before_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    data.backup_rules();
    Ok(())
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    data.pop_backup_rules();
    log::debug!(
      "after visiting rules in {}, rule sched infos: \n{:#?}",
      data.module.name(),
      self.rule_sched_table
    );
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {

    let ops = data.rule().ops.clone();
    let mut new_tl_counter = 0;
    let to_schedule_items: Vec<_> = Self::collect_sched_items(
      ops.into_iter().map(|op| SchedItem::Op(op)),
      &|item, _counter| match item {
        x @ SchedItem::Tl(_, _) => ItemRecursive::Leaf(vec![x]),
        SchedItem::Op(op) => match op.inner() {
          ir::OpEnum::Block(block) => ItemRecursive::Recursive(
            block
              .ops
              .clone()
              .into_iter()
              .map(|op| SchedItem::Op(op))
              .collect(),
          ),
          ir::OpEnum::Timed(ir::TimingIntv { start, end }, op) => {
            ItemRecursive::Recursive(
              std::iter::once(SchedItem::Tl(false, start.clone()))
                .chain(std::iter::once(SchedItem::Op((**op).clone())))
                .chain(std::iter::once(SchedItem::Tl(true, end.clone())))
                .collect(),
            )
          }
          // IfOp should not be recursively flattened, it should be treated as a
          // whole
          // ir::Op::If(ifop) => {}
          _ => ItemRecursive::Leaf(vec![SchedItem::Op(op)]),
        },
      },
      &mut new_tl_counter,
    )
    .collect();

    let (to_schedule_items, _num_cycles) =
      Self::apply_rule_timing(to_schedule_items, data.rule().timing.clone());

    // log::debug!("{:#?}", data.module.values);
    log::debug!(
      "to_schedule_ops: \n{}",
      to_schedule_items
        .iter()
        .enumerate()
        .map(|(i, op)| format!("{}: {}", i, op.dump_with(&data.module.values)))
        .collect::<Vec<_>>()
        .join("\n")
    );

    // let rule_name = data.rule().name().to_string();
    let mut rule_sched_info = RuleSched::default();
    if data.rule().is_single_cycle() {
      rule_sched_info.stat = RuleSchedStat {
        is_combinational: true,
        num_cycles: Some(0),
        guard_always_true: is_true(&mut data.rule().guard_ops.iter()),
      };
      for (idx, sched_item) in to_schedule_items.into_iter().enumerate() {
        if let SchedItem::Op(op) = sched_item {
          rule_sched_info
            .hy_steps
            .entry(TimingExpr::const_(0))
            .or_insert(OpsPerStep {
              time_expr: TimingExpr::const_(0),
              ops: Vec::new(),
            })
            .ops
            .push(IndexedOp {
              idx: Some(idx),
              op: op.clone(),
            });
          rule_sched_info.end_cycle.insert(idx, TimingExpr::const_(0));
        }
      }
    } else {
      anyhow::bail!("multi-cycle rule is not supported");
    }

    let rule_path = data.rule_id();

    let value_context = ValueContext::build(
      &rule_sched_info.hy_steps,
      &rule_sched_info.end_cycle,
      &self.rule_sched_table,
      &self.value_context_table,
      !(rule_sched_info.stat.num_cycles.is_some()
        && rule_sched_info.stat.guard_always_true),
      rule_sched_info.exit_cycle.clone(),
      data,
    )?;

    log::debug!("value_context: \n{}", value_context.dump());

    // add the value_context
    self
      .value_context_table
      .insert(rule_path.clone(), value_context);

    // add the rule_sched_info
    self.rule_sched_table.insert(rule_path, rule_sched_info);
    Ok((vec![data.take_rule()], vec![]))
  }
}

pub enum ItemRecursive {
  Leaf(Vec<SchedItem>),
  Recursive(Vec<SchedItem>),
}

#[derive(Debug, Clone)]
pub enum SchedItem {
  Op(ir::Op),                       // Op(is_or_not, op)
  Tl(bool, Option<ir::TimingExpr>), // Tl(0--open / 1--close, texpr)
}

impl SchedItem {
  pub fn duration(
    &self,
    data: &VisitorData,
    msched: &MSchedPass,
  ) -> Option<u32> {
    match self {
      SchedItem::Op(op) => match op.inner() {
        ir::OpEnum::Invoke(InvokeOp { inst_rule, .. }) => {
          let module_name = data.resolve_path(&inst_rule.path);
          let rule_path: RuleId =
            RuleId::from(&module_name, &inst_rule.rule_name);
          let rule_sched_info = msched.get_sched_info(&rule_path);
          if rule_sched_info.stat.guard_always_true {
            rule_sched_info.stat.num_cycles
          } else {
            None
          }
        }
        _ => Some(0),
      },
      SchedItem::Tl(_, _) => Some(0),
    }
  }

  pub fn dump_with(
    &self,
    value_map: &SlotMap<ir::ValueId, ir::Value>,
  ) -> String {
    match self {
      SchedItem::Op(op) => {
        format!("{}", op.ir_dump_with(value_map))
      }
      SchedItem::Tl(close_or_open, tl) => {
        if *close_or_open {
          format!("{}]", tl.ir_dump_with(value_map))
        } else {
          format!("[{}", tl.ir_dump_with(value_map))
        }
      }
    }
  }
}

impl MSchedPass {
  pub fn new() -> Self {
    Self {
      rule_sched_table: HashMap::new(),
      value_context_table: HashMap::new(),
    }
  }

  pub fn into_inner(self) -> RuleSchedTable {
    self.rule_sched_table
  }

  pub fn get_sched_info(&self, rule_path: &RuleId) -> &RuleSched {
    if let Some(info) = self.rule_sched_table.get(rule_path) {
      info
    } else {
      panic!("rule {} not found", rule_path.to_string());
    }
  }

  fn collect_sched_items(
    items: impl Iterator<Item = SchedItem>,
    flatten_fn: &impl Fn(SchedItem, &mut u32) -> ItemRecursive,
    counter: &mut u32,
  ) -> impl Iterator<Item = SchedItem> {
    let mut collected_items = Vec::new();
    for item in items {
      let recursive = flatten_fn(item, counter);
      match recursive {
        ItemRecursive::Leaf(items) => collected_items.extend(items),
        ItemRecursive::Recursive(items) => collected_items.extend(
          Self::collect_sched_items(items.into_iter(), flatten_fn, counter),
        ),
      }
    }
    collected_items.into_iter()
  }

  fn apply_rule_timing(
    items: Vec<SchedItem>,
    timing: ir::RuleTiming,
  ) -> (Vec<SchedItem>, Option<u32>) {
    match timing {
      ir::RuleTiming::SingleCycle => (items, Some(0)),
      ir::RuleTiming::MultiCycle { num_cycles, intv } => {
        let TimingIntv { start, end } = intv;
        (
          std::iter::once(SchedItem::Tl(false, start))
            .chain(items)
            .chain(std::iter::once(SchedItem::Tl(true, end)))
            .collect(),
          num_cycles,
        )
      }
      _ => unimplemented!(),
    }
  }

  fn setup_directive_deps(
    &mut self,
    items: &[SchedItem],
    data: &VisitorData,
  ) -> DirectiveDepModel {
    let mut dep_model = DirectiveDepModel::new(items, data, self);
    dep_model
  }

  fn setup_defuse_deps(
    &mut self,
    items: &[SchedItem],
    data: &VisitorData,
  ) -> DefUseDepModel {
    let mut dep_model = DefUseDepModel::new(items, data, self);
    dep_model
  }
}

// TODO: the is_true() is a temporary solution (too simple, what about
// true?), should be fixed later
fn is_true(guard: &mut dyn Iterator<Item = &ir::Op>) -> bool {
  guard.next().is_none_or(|op: &ir::Op| {
    if let ir::OpEnum::Lit(ir::LitOp { value, .. }) = op.inner() {
      value.is_true()
    } else {
      false
    }
  })
}
