use std::collections::{HashMap, HashSet};

use super::*;
use crate::passes::Visitor;
use cmtir::to_fir::cmtir_type_to_firrtl_type;
use passes::RuleId;
use slotmap::{SecondaryMap, SparseSecondaryMap};

#[derive(Clone, Debug)]
pub struct ModuleImpl {
  pub wires: SecondaryMap<ir::ValueId, fir::Expression>,
}

#[derive(Clone, Debug)]
pub struct RuleImpl {
  pub implicit_guards: Vec<ir::InstRule>,
  pub pre_body: builder::FirStmtBuilder,
  pub body: builder::FirStmtBuilder,

  pub args: Vec<fir::Expression>,
  pub rsts: Vec<fir::Expression>,
  pub enable: Option<fir::Expression>,
  pub ready: Option<fir::Expression>,
}

impl ToString for RuleImpl {
  fn to_string(&self) -> String {
    let mut buf = String::new();
    buf.push_str(&format!("implicit guards: {:?}\n", self.implicit_guards));
    buf.push_str(&format!(
      "pre_body:{}\n",
      utils::indent(self.pre_body.to_string(), 2)
    ));
    buf.push_str(&format!(
      "body:{}\n",
      utils::indent(self.body.to_string(), 2)
    ));
    buf.push_str(&format!("args: {:?}\n", self.args));
    buf.push_str(&format!("rsts: {:?}\n", self.rsts));
    buf.push_str(&format!("enable: {:?}\n", self.enable));
    buf.push_str(&format!("ready: {:?}\n", self.ready));
    buf
  }
}

pub struct ToFirVisitor {
  pub builder: builder::FirBuilder,
  pub module_impls: HashMap<String, ModuleImpl>,
  pub rule_impls: HashMap<passes::RuleId, RuleImpl>,

  // for every module, maintain a map from valud id to fir expression
  pub value_expr_table: SecondaryMap<ir::ValueId, fir::Expression>,

  // for every module, maintain a set of modules that it includes
  pub module_includes: HashMap<String, HashSet<String>>,

  pub top_synth_modules: HashSet<String>,
}

impl passes::Visitor for ToFirVisitor {
  fn name() -> &'static str {
    "ToFir"
  }

  fn pass_prelog(
    &mut self,
    circuit: &mut crate::Circuit,
  ) -> Result<(), anyhow::Error> {
    // insert all modules to be synthesized into top_synth_modules
    for module in circuit.modules.iter() {
      if module.is_top() {
        log::debug!("add top module: {}", module.name);
        self.top_synth_modules.insert(module.name.clone());
      } else {
        log::debug!("don't add non-top module: {}", module.name);
      }
    }
    Ok(())
  }

  fn prepare_visit_module_impl(
    &mut self,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    self.infer_rule_relations(data)?;
    log::debug!(
      "prepare visiting module: {}, rules: {}",
      data.module.name(),
      data
        .module
        .rules()
        .map(|r| r.name())
        .collect::<Vec<_>>()
        .join(", ")
    );
    Ok(())
  }

  fn before_visit_rules(
    &mut self,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    if data.module.is_ext() {
      self
        .builder
        .push_raw(data.module.name.clone(), data.module.firrtl());

      // update module_includes
      self.module_includes.insert(
        data.module.name().to_string(),
        HashSet::from([data.module.name().to_string()]),
      );

      return Ok(());
    }

    // insert the module to fir-builder
    let name = data.module.name();
    let module_id = self.builder.add_module(name.to_string());

    // update module_includes
    let mut includes = HashSet::new();
    includes.insert(name.to_string());

    for instance in data.module.instances() {
      // extend includes with all modules that are included by the instance
      if let Some(instance_includes) =
        self.module_includes.get(&instance.module)
      {
        includes.extend(instance_includes.iter().cloned());
      }
      if data.module.is_tb() {
        self.top_synth_modules.insert(instance.module.clone());
      }
    }
    self.module_includes.insert(name.to_string(), includes);

    if data.module.is_tb() {
      return Ok(());
    }

    // TODO: these should be added optionally
    self.builder.add_clk_port(module_id);
    self.builder.add_rst_port(module_id);

    // reset the value_expr_table
    self.value_expr_table.clear();

    // add inputs to value_expr_table
    for input in data.module.inputs.iter() {
      self.value_expr_table.insert(
        *input,
        fir::Expression::reference(format!(
          "{}",
          legalize_as_fir_id(data.print_value(*input))
        )),
      );
      let ty = data.type_of(*input).unwrap();
      // add input as port
      self.builder.add_port(
        module_id,
        format!("{}", legalize_as_fir_id(data.print_value(*input))),
        cmtir_type_to_firrtl_type(ty),
        fir::Direction::Input,
      );
      // invalidate input
      self.builder.add_defs(
        module_id,
        vec![fir::Statement::invalidate(fir::Expression::reference(
          format!("{}", legalize_as_fir_id(data.print_value(*input))),
        ))],
      );
    }

    // add output as port
    for output in data.module.outputs.iter() {
      let ty = data.type_of(*output).unwrap();
      self.builder.add_port(
        module_id,
        format!("{}", legalize_as_fir_id(data.print_value(*output))),
        cmtir_type_to_firrtl_type(ty),
        fir::Direction::Output,
      );
      // invalidate output
      self.builder.add_defs(
        module_id,
        vec![fir::Statement::invalidate(fir::Expression::reference(
          format!("{}", legalize_as_fir_id(data.print_value(*output))),
        ))],
      );
    }

    // add instances
    for instance in data.module.instances() {
      self.builder.add_instance(
        module_id,
        legalize_as_fir_id(instance.name.clone()),
        instance.module.clone(),
      );

      let inst_module = data.circuit.module(&instance.module).unwrap();

      // invalidate all inputs of the instance
      if let Some(ext_info) = inst_module.ext_body() {
        for (i, _input) in inst_module.inputs().enumerate() {
          self.builder.add_defs(
            module_id,
            vec![fir::Statement::invalidate(fir::Expression::reference(
              format!(
                "{}.{}",
                legalize_as_fir_id(instance.name.clone()),
                legalize_as_fir_id(ext_info.bindings.inputs[i].clone())
              ),
            ))],
          );
        }
        // assign all zero to enables
        for rule in inst_module.rules() {
          if rule.is_method() {
            if let Some(ref enable) = rule.enable {
              self.builder.add_defs(
                module_id,
                vec![fir::Statement::connect(
                  fir::Expression::reference(format!(
                    "{}.{}",
                    legalize_as_fir_id(instance.name.clone()),
                    legalize_as_fir_id(enable.clone())
                  )),
                  fir::Expression::zeros(1),
                )],
              )
            }
          }
        }
      } else {
        for input in inst_module.inputs() {
          self.builder.add_defs(
            module_id,
            vec![fir::Statement::invalidate(fir::Expression::reference(
              format!(
                "{}.{}",
                legalize_as_fir_id(instance.name.clone()),
                legalize_as_fir_id(
                  data.print_value_in_module(input, inst_module)
                )
              ),
            ))],
          );
        }
        for rule in inst_module.rules() {
          if rule.is_method() && !rule.is_private() {
            self.builder.add_defs(
              module_id,
              // vec![fir::Statement::invalidate(fir::Expression::reference(
              //   format!(
              //     "{}.{}_enable",
              //     legalize_as_fir_id(instance.name.clone()),
              //     legalize_as_fir_id(rule.name().to_string())
              //   ),
              // ))],
              vec![fir::Statement::connect(
                fir::Expression::reference(format!(
                  "{}.{}_enable",
                  legalize_as_fir_id(instance.name.clone()),
                  legalize_as_fir_id(rule.name().to_string())
                )),
                fir::Expression::zeros(1),
              )],
            );
          }
        }
      }

      if let Some(ext_info) = inst_module.ext_body() {
        if let Some(clock) = ext_info.clock.clone() {
          self.builder.add_connection(
            module_id,
            fir::Expression::reference(format!(
              "{}.{}",
              legalize_as_fir_id(instance.name.clone()),
              legalize_as_fir_id(clock)
            )),
            fir::Expression::reference(format!("clk")),
          );
        }
        if let Some(reset) = ext_info.reset.clone() {
          self.builder.add_connection(
            module_id,
            fir::Expression::reference(format!(
              "{}.{}",
              legalize_as_fir_id(instance.name.clone()),
              legalize_as_fir_id(reset)
            )),
            fir::Expression::reference(format!("rst")),
          );
        }
      } else {
        // connect clock and reset
        self.builder.add_connection(
          module_id,
          fir::Expression::reference(format!(
            "{}.clk",
            legalize_as_fir_id(instance.name.clone())
          )),
          fir::Expression::reference(format!("clk")),
        );
        self.builder.add_connection(
          module_id,
          fir::Expression::reference(format!(
            "{}.rst",
            legalize_as_fir_id(instance.name.clone())
          )),
          fir::Expression::reference(format!("rst")),
        );
      }
    }

    if data.module.is_native() {
      // add enable/ready port
      for rule in data.module.rules() {
        if rule.is_method() && !rule.is_private() {
          let rule_name = rule.name().to_string();
          self.builder.add_port(
            module_id,
            format!("{}_enable", legalize_as_fir_id(rule_name.clone())),
            fir::Type::uint(1),
            fir::Direction::Input,
          );
          self.builder.add_port(
            module_id,
            format!("{}_ready", legalize_as_fir_id(rule_name.clone())),
            fir::Type::uint(1),
            fir::Direction::Output,
          );
        }
      }
    }

    Ok(())
  }

  fn get_rules_in_order(&mut self, data: &mut VisitorData) -> Vec<ir::Rule> {
    // rules should be visited in the topological order
    self.get_rules_in_topo_order(data)
  }

  /// Create an rule implementation for the given rule
  fn visit_rule_impl(
    &mut self,
    data: &mut passes::VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    if data.module.is_tb() {
      return Ok((vec![data.take_rule()], vec![]));
    }

    let rule_name = data.rule().name().to_string();

    let ext_info = if let Some(ext_info) = data.module.ext_body() {
      let mut input_map = SparseSecondaryMap::new();
      let mut output_map = SparseSecondaryMap::new();
      for (i, input) in data.module.inputs().enumerate() {
        input_map.insert(input, ext_info.bindings.inputs[i].clone());
      }
      for (i, output) in data.module.outputs().enumerate() {
        output_map.insert(output, ext_info.bindings.outputs[i].clone());
      }
      Some(ExtInfo {
        input_map,
        output_map,
      })
    } else {
      None
    };

    let rule_impl = self.impl_rule(data, ext_info)?;
    self
      .rule_impls
      .insert(data.rule_id_by_rule_name(&rule_name), rule_impl);
    Ok((vec![data.take_rule()], vec![]))
  }

  fn after_visit_rules(
    &mut self,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    if data.module.is_ext() || data.module.is_tb() {
      return Ok(());
    }

    let rule_order = self.rule_schedule_order(data)?;

    log::debug!(
      "\trule schedule for module {}: {:?}",
      data.module.name(),
      rule_order
    );

    self.glue_rules(rule_order, data)?;

    self.module_impls.insert(
      data.module.name().to_string(),
      ModuleImpl {
        wires: self.value_expr_table.clone(),
      },
    );
    Ok(())
  }

  fn after_visit_modules(
    &mut self,
    circuit: &mut crate::Circuit,
  ) -> Result<(), anyhow::Error> {
    let _ = circuit;

    self
      .top_synth_modules
      .retain(|m| circuit.module(m).unwrap().to_be_synthesize());
    Ok(())
  }
}

struct ExtInfo {
  input_map: SparseSecondaryMap<ir::ValueId, String>,
  output_map: SparseSecondaryMap<ir::ValueId, String>,
}

impl ToFirVisitor {
  pub fn new() -> Self {
    Self {
      builder: builder::FirBuilder::new(),
      module_impls: HashMap::new(),
      rule_impls: HashMap::new(),
      value_expr_table: SecondaryMap::new(),
      module_includes: HashMap::new(),
      top_synth_modules: HashSet::new(),
    }
  }

  pub fn build(self) -> Vec<(String, String)> {
    log::info!(
      "top synth modules: {}",
      self
        .top_synth_modules
        .iter()
        .map(|m| m.to_string())
        .collect::<Vec<_>>()
        .join(", ")
    );
    let mut circuits = Vec::new();
    for module in self.top_synth_modules.iter() {
      if !self.module_includes.contains_key(module) {
        log::error!("module includes does not contain: {}", module);
      }
      circuits.push((
        module.clone(),
        self
          .builder
          .build(module.clone(), self.module_includes[module].iter().cloned()),
      ));
    }
    circuits
  }

  fn impl_rule(
    &mut self,
    data: &mut passes::VisitorData,
    ext_info: Option<ExtInfo>,
  ) -> Result<RuleImpl, anyhow::Error> {
    if data.rule().name().starts_with("_feed_arg_") {
      return Ok(RuleImpl {
        implicit_guards: vec![],
        pre_body: builder::FirStmtBuilder::new(),
        body: builder::FirStmtBuilder::new(),
        args: data
          .rule()
          .signature
          .inputs()
          .map(|value_id| self.expr_by_value(value_id, data))
          .collect::<Result<Vec<_>, anyhow::Error>>()?,
        rsts: vec![],
        enable: None,
        ready: None,
      });
    }

    if data.rule().name().starts_with("_get_res_") {
      return Ok(RuleImpl {
        implicit_guards: vec![],
        pre_body: builder::FirStmtBuilder::new(),
        body: builder::FirStmtBuilder::new(),
        args: vec![],
        rsts: data
          .rule()
          .signature
          .outputs()
          .map(|value_id| self.expr_by_value(value_id, data))
          .collect::<Result<Vec<_>, anyhow::Error>>()?,
        enable: None,
        ready: None,
      });
    }

    // get implicit guards:
    // for every invoke, the invoked method's guard is an implicit guard of the
    // current rule
    let invokes = Self::filter_op(data.rule().ops(), |op| {
      matches!(op.inner(), ir::OpEnum::Invoke(_))
    });
    let mut implicit_guards = Vec::new();
    for op in invokes {
      op.as_invoke_op_ref()
        .map(|invoke| {
          implicit_guards.push(invoke.inst_rule.clone());
        })
        .ok_or(data.report_error(format!(
          "{} is not an invoke op",
          op.ir_dump_with(&data.module.values)
        )))?;
    }

    let mut pre_body = builder::FirStmtBuilder::new();
    let mut body = builder::FirStmtBuilder::new();

    let rule = data.rule();
    let ir::Rule {
      is_ext,
      name,
      signature,
      // timing,
      enable,
      ready,
      guard_ops,
      ops,
      ..
    } = rule.clone();

    // iterate over rule's guard ops
    for op in guard_ops {
      self.impl_op(op, &mut pre_body, data)?;
    }

    if pre_body.values.len() > 1 {
      return Err(data.report_error(format!(
        "guard must have single/zero output, got {}",
        pre_body.values.len()
      )))?;
    }

    for op in ops {
      // log::debug!("\t\timpl op: {}", op.ir_dump_with(&data.module.values));
      self.impl_op(op, &mut body, data)?;
    }

    let (args, rsts) = if let Some(ext_info) = ext_info {
      (
        signature
          .inputs()
          .map(|value_id| {
            fir::Expression::reference(
              ext_info.input_map.get(value_id).unwrap().clone(),
            )
          })
          .collect(),
        signature
          .outputs()
          .map(|value_id| {
            fir::Expression::reference(
              ext_info.output_map.get(value_id).unwrap().clone(),
            )
          })
          .collect(),
      )
    } else {
      self.bind_outputs(signature.outputs().collect(), &mut body, data)?;
      (
        signature
          .inputs()
          .map(|value_id| self.expr_by_value(value_id, data))
          .collect::<Result<Vec<_>, anyhow::Error>>()?,
        signature
          .outputs()
          .map(|value_id| self.expr_by_value(value_id, data))
          .collect::<Result<Vec<_>, anyhow::Error>>()?,
      )
    };

    let enable = if is_ext {
      enable.map(|name| {
        fir::Expression::reference(legalize_as_fir_id(name.clone()))
      })
    } else {
      if signature.is_method() {
        Some(fir::Expression::reference(format!(
          "{}_enable",
          legalize_as_fir_id(name.clone())
        )))
      } else {
        None
      }
    };
    let ready = if is_ext {
      ready.map(|name| {
        fir::Expression::reference(legalize_as_fir_id(name.clone()))
      })
    } else {
      if signature.is_method() {
        Some(fir::Expression::reference(format!(
          "{}_ready",
          legalize_as_fir_id(name.clone())
        )))
      } else {
        None
      }
    };

    Ok(RuleImpl {
      implicit_guards,
      pre_body,
      body,
      enable,
      ready,
      args,
      rsts,
    })
  }

  fn bind_outputs(
    &mut self,
    outputs: Vec<ir::ValueId>,
    body: &mut builder::FirStmtBuilder,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let values = body.take_values();
    if values.len() < outputs.len() {
      log::error!(
        "[{}:{}] mismatch number of outputs: {} vs {}, the last op is\n{}",
        data.module.name(),
        data.rule().name(),
        values.len(),
        outputs.len(),
        data
          .rule()
          .ops()
          .last()
          .map(|op| op.ir_dump_with(&data.module.values))
          .unwrap_or_else(|| "None".to_string())
      );
      panic!(
        "mismatch number of outputs: {} vs {}",
        values.len(),
        outputs.len()
      );
    }

    for (value, expr) in outputs.into_iter().zip(values.into_iter()) {
      let value_expr = self.expr_by_value(value, data)?;
      body.register_expr(value, &value_expr, data);
      body.connect(value_expr.clone(), expr);
      body.values.push(value_expr);
    }
    Ok(())
  }

  fn impl_op(
    &mut self,
    op: ir::Op,
    body: &mut builder::FirStmtBuilder,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let outputs: Vec<_> = op.outputs().collect();

    match op.inner {
      OpEnum::Nop(_) => {}
      OpEnum::Assign(ir::AssignOp { res, value }) => {
        let rhs_expr = self.expr_by_value(value, data)?;
        self.bridge_value_expr(res, rhs_expr)?;
      }
      OpEnum::Lit(ir::LitOp { res, value }) => {
        // self.bridge_value_expr(res, value.to_fir_lit())?
        body.add_lit(res, value.to_fir_lit(), self, data)?;
      }
      OpEnum::Cmp(ir::CmpOp { res, cmp, a, b }) => {
        body.add_prim(res, cmp.into(), vec![a, b], vec![], self, data)?;
      }
      OpEnum::Prim(ir::PrimOp {
        res,
        prim,
        inputs,
        attrs,
      }) => {
        body.add_prim(res, prim.into(), inputs, attrs, self, data)?;
      }
      OpEnum::Invoke(ir::InvokeOp {
        res,
        inst_rule,
        args,
      }) => {
        body.add_invoke(res, inst_rule.canonicalize(), args, self, data)?;
      }
      OpEnum::Call(_) => {
        unreachable!("this is not allowed, callop must be inlined")
      }
      OpEnum::Timed(_, _) => {
        return Err(data.report_error(format!(
          "this is not allowed, since single-cycle rule doesn't have timing info: {}",
          op.ir_dump_with(&data.module.values)
        )))?;
      }
      OpEnum::Block(op) => {
        for op in op.ops {
          self.impl_op(op, body, data)?;
        }
      }
      OpEnum::If(op) => {
        let mut then_fir = builder::FirStmtBuilder::new();
        self.impl_op(op.then_body, &mut then_fir, data)?;
        let else_fir = if let Some(else_body) = op.else_body {
          let mut else_fir = builder::FirStmtBuilder::new();
          self.impl_op(else_body, &mut else_fir, data)?;
          Some(else_fir)
        } else {
          None
        };
        body.push_when(op.res, op.cond, then_fir, else_fir, self, data)?;
      }
      OpEnum::Field(ir::FieldOp { res, value, field }) => {
        body.add_field(res, value, field, self, data)?;
      }
      OpEnum::Return(_) => {}
      OpEnum::Aggregate(ir::AggregateOp {
        res,
        values,
        fields,
      }) => {
        body.add_aggregate(res, values, fields, self, data)?;
      }
      OpEnum::Delay(ir::DelayOp { .. }) => {
        return Err(data.report_error(format!(
          "this is not allowed, since delay-op should be moved behind to-fir pass: {}",
          op.ir_dump_with(&data.module.values)
        )))?;
      }
      OpEnum::DynDelay(ir::DynDelayOp { .. }) => {
        return Err(data.report_error(format!(
          "this is not allowed, since dyn-delay-op should be moved behind to-fir pass: {}",
          op.ir_dump_with(&data.module.values)
        )))?;
      }
      _ => {
        unreachable!(
          "unsupported op: {}",
          op.ir_dump_with(&data.module.values)
        );
      }
    }

    body.set_values(outputs.into_iter(), self, data)
  }

  fn bridge_value_expr(
    &mut self,
    value: ir::ValueId,
    expr: fir::Expression,
  ) -> Result<(), anyhow::Error> {
    self.value_expr_table.insert(value, expr.clone());
    Ok(())
  }

  fn add_value_expr(
    &mut self,
    value: ir::ValueId,
    data: &mut passes::VisitorData,
  ) -> fir::Expression {
    let expr =
      fir::Expression::reference(legalize_as_fir_id(data.print_value(value)));
    self.value_expr_table.insert(value, expr.clone());
    expr
  }

  pub fn expr_by_value(
    &mut self,
    value: ir::ValueId,
    data: &mut passes::VisitorData,
  ) -> Result<fir::Expression, anyhow::Error> {
    if let Some(expr) = self.value_expr_table.get(value) {
      Ok(expr.clone())
    } else {
      let expr = self.add_value_expr(value, data);
      Ok(expr)
    }
  }

  fn rule_schedule_order(
    &mut self,
    data: &mut passes::VisitorData,
  ) -> Result<Vec<InstRule>, anyhow::Error> {
    let rules = data
      .module
      .schedules()
      .next()
      .ok_or(data.report_error("no schedule found".to_string()))?;
    // log::debug!("\tsched orders: {:?}", rules);

    Ok(rules)
  }

  fn add_rule_to_firbuilder(
    &mut self,
    module_id: usize,
    name: String,
    enable: Option<fir::Expression>,
    ready: Option<fir::Expression>,
    mut pre_body: builder::FirStmtBuilder,
    body: builder::FirStmtBuilder,
    data: &mut passes::VisitorData,
  ) -> Result<(Option<fir::Expression>, fir::Expression), anyhow::Error> {
    let pre_body_values = pre_body.take_values();
    let guard: Option<fir::Expression> = if pre_body_values.len() == 0 {
      None
    } else if pre_body_values.len() == 1 {
      Some(pre_body_values.into_iter().next().unwrap())
    } else {
      return Err(data.report_error(format!(
        "unsupported pre-body: {:?}",
        pre_body_values
      )))?;
    };

    let (vars, stmts) = pre_body.into_inner_split();

    self.builder.add_statements(module_id, stmts);
    self.builder.add_defs(module_id, vars);

    // create a fire signal
    let fire_name = format!("{}_fire", name);
    let fire_expr =
      self
        .builder
        .add_var_def(module_id, fire_name, fir::Type::uint(1), false);

    self.builder.add_connection(
      module_id,
      fire_expr.clone(),
      if let Some(enable) = enable {
        if let Some(guard) = guard.clone() {
          fir::Expression::and(guard, enable)
        } else {
          enable
        }
      } else {
        guard.clone().unwrap_or(fir::Expression::ones(1))
      },
    );

    if let Some(ready) = ready.clone() {
      self.builder.add_connection(
        module_id,
        ready,
        guard.unwrap_or(fir::Expression::ones(1)),
      );
    }

    let (vars, stmts) = body.into_inner_split();
    self.builder.add_defs(module_id, vars);
    self
      .builder
      .add_cond(module_id, fire_expr.clone(), stmts.into());

    Ok((ready, fire_expr))
  }

  fn glue_rules(
    &mut self,
    rule_order: Vec<InstRule>,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let mut private_rule_ready_table: HashMap<String, fir::Expression> =
      HashMap::new();
    let scheduled_rule_set = rule_order
      .clone()
      .into_iter()
      .map(|inst_rule| inst_rule.rule_name.clone())
      .collect::<HashSet<_>>();

    let private_rules = Self::topo_sort_rules(
      data
        .module
        .rules()
        .filter(|rule| {
          !scheduled_rule_set.contains(rule.name()) && {
            // skip _feed_arg, _get_res, _read_arg, _write_res
            !rule.name().starts_with("_feed_arg_")
              && !rule.name().starts_with("_get_res_")
              && !rule.name().starts_with("_read_arg_")
              && !rule.name().starts_with("_write_res_")
          }
        })
        .collect::<Vec<_>>()
        .into_iter(),
      |_i, r| (r.name().to_string(), r.name().to_string()),
    );

    for rule_name in private_rules {
      let RuleImpl {
        implicit_guards,
        mut pre_body,
        body,
        enable,
        ready,
        ..
      } = self
        .rule_impls
        .get(&RuleId {
          module_name: data.module.name().to_string(),
          rule_name: rule_name.clone(),
        })
        .ok_or(data.report_error(format!("rule {}'s impl not found", rule_name)))?
        .clone();

      if let Some(enable) = enable.clone() {
        // add enable to the builder
        pre_body.register_expr_with_type(enable, ir::Type::uint(1));
      }

      if let Some(ready) = ready.clone() {
        // add ready to the builder
        pre_body.register_expr_with_type(ready, ir::Type::uint(1));
      }

      let &module_id =
        self.builder.module_table.get(data.module.name()).unwrap();

      let name = legalize_as_fir_id(rule_name.clone());

      for (i, implicit_guard) in implicit_guards.iter().enumerate() {
        self.eval_guard(
          format!("{}_{}", name, i),
          &mut pre_body,
          &ir::InstRule::new(vec![rule_name.clone()]),
          implicit_guard.clone(),
          None,
          &private_rule_ready_table,
          data,
        )?;
      }

      let (ready, _) = self.add_rule_to_firbuilder(
        module_id, name, enable, ready, pre_body, body, data,
      )?;
      if let Some(ready) = ready {
        // TODO: add warning here?
        private_rule_ready_table.insert(rule_name.clone(), ready);
      }
    }

    // glue rules in schedule order
    let rules_to_be_glued = rule_order;
    let mut done_rules: Vec<(InstRule, fir::Expression)> = Vec::new();

    for inst_rule in rules_to_be_glued {
      let inst_rule = inst_rule.canonicalize();
      if !inst_rule.is_self() {
        return Err(data.report_error(format!(
          "rule {} is not a self rule, but is in schedule order",
          inst_rule.to_string()
        )))?;
      }
      log::debug!("\tgluing rule: {}", inst_rule.to_string());

      let inst_module_name = data.resolve_path(&inst_rule.path);

      log::debug!(
        "\t\t[module_name] {}, [rule_name] {}",
        inst_module_name,
        inst_rule.rule_name
      );

      let RuleImpl {
        mut implicit_guards,
        mut pre_body,
        body,
        enable,
        ready,
        ..
      } = self
        .rule_impls
        .get(&RuleId {
          module_name: inst_module_name.clone(),
          rule_name: inst_rule.rule_name.clone(),
        })
        .ok_or(data.report_error(format!(
          "rule {}'s impl not found",
          inst_rule.to_string()
        )))?
        .clone();

      let &module_id =
        self.builder.module_table.get(data.module.name()).unwrap();

      let name = legalize_as_fir_id(inst_rule.to_string());

      implicit_guards.push(inst_rule.clone());

      for (i, implicit_guard) in implicit_guards.iter().enumerate() {
        self.eval_guard(
          format!("{}_{}", name, i),
          &mut pre_body,
          &inst_rule,
          implicit_guard.clone(),
          Some(&done_rules),
          &private_rule_ready_table,
          data,
        )?;
      }

      let (_, fire_expr) = self.add_rule_to_firbuilder(
        module_id, name, enable, ready, pre_body, body, data,
      )?;

      done_rules.push((inst_rule, fire_expr));
    }

    Ok(())
  }

  fn eval_guard(
    &mut self,
    name: String,
    body: &mut builder::FirStmtBuilder,
    current_inst_rule: &ir::InstRule,
    ig_inst_rule: ir::InstRule,
    done_rules: Option<&Vec<(InstRule, fir::Expression)>>,
    private_rule_ready_table: &HashMap<String, fir::Expression>,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    log::debug!("\t\teval guard: {}", ig_inst_rule.to_string());

    let ig_inst_rule = ig_inst_rule.canonicalize();

    // guards of ig_inst_rule
    if ig_inst_rule.is_self() {
      if ig_inst_rule.rule_name != current_inst_rule.rule_name {
        if let Some(ready) =
          private_rule_ready_table.get(&ig_inst_rule.rule_name)
        {
          body.value_and(name.clone(), ready.clone())?;
        } else {
          // TODO: add warning here?
          // log::warn!("rule {}'s ready not found", ig_inst_rule.to_string());
        }
      }
    } else {
      // value_and ready
      let RuleImpl { ready, .. } = self
        .rule_impls
        .get(&data.rule_id_by_inst_rule(&ig_inst_rule))
        .ok_or(data.report_error(format!(
          "rule {}'s impl not found",
          ig_inst_rule.to_string()
        )))?
        .clone();
      if let Some(ready) = ready {
        // add ig_inst_rule.path as ready's prefix
        let ready = ready
          .prefix_with(&legalize_as_fir_id(ig_inst_rule.path.join(".")), ".");
        body.value_and(name.clone(), ready.clone())?;
      } else {
        // TODO: add warning here?
        // log::warn!("rule {}'s ready not found", ig_inst_rule.to_string());
      }
    }

    // conflicts with before done rules
    for (i, (done_inst_rule, done_fire)) in
      done_rules.into_iter().flatten().enumerate()
    {
      if data.check_conflict(done_inst_rule.clone(), ig_inst_rule.clone()) {
        body.value_and_not(format!("{}_{}", name, i), done_fire.clone())?;

        log::warn!(
          "\t\t[{}] rule {} will be blocked by the fire of {}",
          name,
          ig_inst_rule.to_string(),
          done_inst_rule.to_string(),
        );
      }
    }

    Ok(())
  }
}
