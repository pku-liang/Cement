use std::collections::HashSet;

use super::*;
use cmtir::to_fir::cmtir_type_to_firrtl_type;
use fir::Statement;
use indexmap::IndexMap;
use transform::RuleImpl;

#[derive(Clone)]
pub enum FirModule {
  Module(fir::Module),
  Raw(String),
}

impl FirModule {
  pub fn as_module_mut(&mut self) -> &mut fir::Module {
    match self {
      FirModule::Module(module) => module,
      FirModule::Raw(_) => panic!("Raw module is not a fir::Module"),
    }
  }

  pub fn as_raw_mut(&mut self) -> &mut String {
    match self {
      FirModule::Module(_) => panic!("Module is not a raw string"),
      FirModule::Raw(raw) => raw,
    }
  }
}

/// Context for building FIRRTL
pub struct FirBuilder {
  pub fir_modules: Vec<FirModule>,
  pub module_table: IndexMap<String, usize>,
}

impl FirBuilder {
  pub fn new() -> Self {
    Self {
      fir_modules: Vec::new(),
      module_table: IndexMap::new(),
    }
  }

  pub fn build(
    &self,
    main: String,
    modules: impl Iterator<Item = String>,
  ) -> String {
    let mut raw = String::new();
    let mut circuit = fir::Circuit::new();
    circuit.main = Some(main);
    for module in modules {
      let module_id = self.module_table[&module];
      match self.fir_modules[module_id].clone() {
        FirModule::Module(module) => circuit.modules.push(module),
        FirModule::Raw(s) => raw.push_str(&s),
      }
    }
    format!("{}{}", circuit.to_string(), raw)
  }

  pub fn push_raw(&mut self, name: String, raw: String) {
    let raw = utils::indent(raw.replace("###", ""), 2);

    let module_id = self.fir_modules.len();
    self.fir_modules.push(FirModule::Raw(raw));
    self.module_table.insert(name, module_id);
  }

  pub fn add_clk_port(&mut self, module_id: usize) {
    self.add_port(
      module_id,
      "clk".to_string(),
      fir::Type::ClockType,
      fir::Direction::Input,
    );
  }

  pub fn add_rst_port(&mut self, module_id: usize) {
    self.add_port(
      module_id,
      "rst".to_string(),
      fir::Type::ResetType,
      fir::Direction::Input,
    );
  }

  pub fn add_defs(&mut self, module_id: usize, defs: Vec<fir::Statement>) {
    let defs = defs
      .into_iter()
      .filter(|stmt| {
        // if a variable is also a port, skip it
        if let fir::Statement::DefWire { name, .. } = stmt {
          !self.fir_modules[module_id]
            .as_module_mut()
            .ports
            .iter()
            .any(|p| &p.name == name)
        } else {
          true
        }
      })
      .collect::<Vec<_>>();
    self.fir_modules[module_id]
      .as_module_mut()
      .defs
      .extend(defs);
  }

  pub fn add_statements(
    &mut self,
    module_id: usize,
    stmts: Vec<fir::Statement>,
  ) {
    self.fir_modules[module_id]
      .as_module_mut()
      .body
      .extend(stmts);
  }

  pub fn add_connection(
    &mut self,
    module_id: usize,
    src: fir::Expression,
    dst: fir::Expression,
  ) {
    self.fir_modules[module_id]
      .as_module_mut()
      .body
      .push(fir::Statement::connect(src, dst));
  }

  pub fn add_ports(&mut self, module_id: usize, ports: Vec<fir::Port>) {
    self.fir_modules[module_id]
      .as_module_mut()
      .ports
      .extend(ports);
  }

  pub fn add_port(
    &mut self,
    module_id: usize,
    port_name: String,
    port_type: fir::Type,
    port_dir: fir::Direction,
  ) {
    self.fir_modules[module_id]
      .as_module_mut()
      .ports
      .push(fir::Port::new(port_name, port_type, port_dir));
  }

  pub fn add_var_def(
    &mut self,
    module_id: usize,
    var_name: String,
    var_type: fir::Type,
    prefix: bool,
  ) -> fir::Expression {
    let expr = fir::Expression::reference(legalize_as_fir_id(var_name.clone()));
    self.fir_modules[module_id].as_module_mut().defs.push(
      fir::Statement::def_wire(
        legalize_as_fir_id(var_name.clone()),
        var_type,
        prefix,
      ),
    );
    expr
  }

  pub fn add_cond(
    &mut self,
    module_id: usize,
    cond: fir::Expression,
    stmt: fir::Statement,
  ) {
    self.fir_modules[module_id]
      .as_module_mut()
      .body
      .push(fir::Statement::when(cond, stmt, fir::Statement::EmptyStmt));
  }

  pub fn add_module(&mut self, module_name: String) -> usize {
    let module_id = self.fir_modules.len();
    let mut module = fir::Module::new(module_name.to_owned());
    module.body = fir::Statement::block(vec![]);
    self.fir_modules.push(FirModule::Module(module));
    self.module_table.insert(module_name, module_id);
    module_id
  }

  pub fn add_instance(
    &mut self,
    module_id: usize,
    inst_var_name: String,
    inst_module_name: String,
  ) {
    self.fir_modules[module_id].as_module_mut().defs.push(
      fir::Statement::def_inst(
        legalize_as_fir_id(inst_var_name),
        inst_module_name,
      ),
    );
  }
}

pub fn legalize_as_fir_id(id: String) -> String {
  let id = id.replace('.', "_");
  let id = id.replace('+', "p");
  id
}

#[derive(Debug, Clone)]
pub struct FirStmtBuilder {
  pub stmts: Vec<fir::Statement>,
  pub vars: Vec<fir::Statement>,
  pub values: Vec<fir::Expression>,
}

impl ToString for FirStmtBuilder {
  fn to_string(&self) -> String {
    let mut buf = String::new();
    buf.push_str(
      &self
        .vars
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n"),
    );
    buf.push('\n');
    buf.push_str(
      &self
        .stmts
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n"),
    );
    buf.push('\n');
    buf.push_str(&format!(
      "values: {}",
      self
        .values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
    ));
    buf
  }
}

impl FirStmtBuilder {
  pub fn new() -> Self {
    Self {
      stmts: Vec::new(),
      vars: Vec::new(),
      values: Vec::new(),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.stmts.is_empty() && self.vars.is_empty()
  }

  pub fn into_inner_split(self) -> (Vec<fir::Statement>, Vec<fir::Statement>) {
    (self.vars, self.stmts)
  }

  pub fn into_inner(mut self) -> Vec<fir::Statement> {
    self.vars.extend(self.stmts);
    self.vars
  }

  pub fn push_stmt(&mut self, stmt: fir::Statement) {
    self.stmts.push(stmt);
  }

  pub fn push_var(
    &mut self,
    var_name: String,
    var_type: fir::Type,
    prefix: bool,
  ) -> fir::Expression {
    if self.vars.iter().any(|stmt| match stmt {
      fir::Statement::DefWire { name, .. } => name == &var_name,
      _ => false,
    }) {
      return fir::Expression::reference(legalize_as_fir_id(var_name.clone()));
    }

    self.vars.push(fir::Statement::def_wire(
      legalize_as_fir_id(var_name.clone()),
      var_type,
      prefix,
    ));
    self
      .vars
      .push(fir::Statement::invalidate(fir::Expression::reference(
        legalize_as_fir_id(var_name.clone()),
      )));
    fir::Expression::Reference {
      prefix,
      name: legalize_as_fir_id(var_name.clone()),
    }
  }

  pub fn take_values(&mut self) -> Vec<fir::Expression> {
    std::mem::take(&mut self.values)
  }

  pub fn set_values(
    &mut self,
    values: impl Iterator<Item = ir::ValueId>,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    self.values.clear();
    for value in values {
      let expr = visitor.expr_by_value(value, data)?;
      self.values.push(expr);
    }
    Ok(())
  }

  pub fn value_and(
    &mut self,
    name: String,
    rhs: fir::Expression,
  ) -> Result<(), anyhow::Error> {
    let lhs = self.take_values();
    if lhs.len() == 0 {
      self.values.push(rhs);
    } else if lhs.len() == 1 {
      let lhs = lhs[0].clone();
      if lhs != rhs {
        let new_lhs = self.push_var(name, fir::Type::uint(1), false);
        self.connect(new_lhs.clone(), fir::Expression::and(lhs, rhs));
        self.values.push(new_lhs);
      }
    } else {
      return Err(anyhow::anyhow!("unsupported value_and: {}", lhs.len()));
    }
    Ok(())
  }

  pub fn value_and_not(
    &mut self,
    name: String,
    rhs: fir::Expression,
  ) -> Result<(), anyhow::Error> {
    let lhs = self.take_values();
    if lhs.len() == 0 {
      let not_rhs = self.push_var(name, fir::Type::uint(1), false);
      self.connect(not_rhs.clone(), fir::Expression::not(rhs));
      self.values.push(not_rhs);
    } else if lhs.len() == 1 {
      let lhs = lhs[0].clone();
      // log::debug!("\t\tvalue_and_not: lhs: {:?}, rhs: {:?}", lhs, rhs);
      if lhs != rhs {
        let new_lhs = self.push_var(name, fir::Type::uint(1), false);
        self.connect(
          new_lhs.clone(),
          fir::Expression::and(lhs, fir::Expression::not(rhs)),
        );
        self.values.push(new_lhs);
      } else {
        self.values.push(fir::Expression::zeros(1));
      }
    } else {
      return Err(anyhow::anyhow!("unsupported guard lhs: {:?}", lhs));
    }
    Ok(())
  }

  pub fn register_expr_with_type(
    &mut self,
    expr: fir::Expression,
    ty: ir::Type,
  ) {
    match expr {
      fir::Expression::Reference { name, prefix: _ } => {
        self.push_var(
          legalize_as_fir_id(name.clone()),
          cmtir_type_to_firrtl_type(ty),
          false,
        );
      }
      _ => {
        panic!("unsupported expr: {:?}", expr);
      }
    }
  }

  pub fn register_expr(
    &mut self,
    value: ir::ValueId,
    expr: &fir::Expression,
    data: &mut passes::VisitorData,
  ) {
    match expr {
      fir::Expression::Reference { name, prefix: _ } => {
        self.push_var(
          legalize_as_fir_id(name.clone()),
          {
            if let Some(ty) = data.type_of(value) {
              cmtir_type_to_firrtl_type(ty)
            } else {
              panic!(
                "undefined type of value: {}, in {}::{}:\n{}",
                data.print_value(value),
                data.module.name(),
                data.rule().name(),
                data.rule().ir_dump_with(&data.module.values)
              );
            }
          },
          false,
        );
      }
      _ => {
        panic!("unsupported expr: {:?}", expr)
      }
    }
  }

  pub fn connect(&mut self, lhs: fir::Expression, rhs: fir::Expression) {
    self.push_stmt(fir::Statement::connect(lhs, rhs));
  }

  pub fn assign_to(&mut self, lhs: Vec<fir::Expression>) {
    let values = std::mem::take(&mut self.values);
    for (lhs, rhs) in lhs.clone().into_iter().zip(values) {
      self.connect(lhs, rhs);
    }
    self.values = lhs;
  }

  pub fn add_lit(
    &mut self,
    lhs: ir::ValueId,
    rhs: fir::Expression,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let res_expr = visitor.expr_by_value(lhs, data)?;
    self.register_expr(lhs, &res_expr, data);
    self.connect(res_expr, rhs);
    Ok(())
  }

  pub fn add_prim(
    &mut self,
    res: ir::ValueId,
    prim: fir::PrimOp,
    args: Vec<ir::ValueId>,
    attrs: Vec<u32>,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let args_expr = args
      .iter()
      .map(|arg| visitor.expr_by_value(*arg, data))
      .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let res_expr = visitor.expr_by_value(res, data)?;
    self.register_expr(res, &res_expr, data);
    let cmp_expr: fir::Expression = fir::Expression::do_prim(
      prim,
      args_expr,
      attrs.into_iter().map(|x| x as usize).collect(),
    );
    self.push_stmt(fir::Statement::connect(res_expr, cmp_expr));

    Ok(())
  }

  pub fn add_invoke(
    &mut self,
    rsts: Vec<ir::ValueId>,
    inst_rule: ir::InstRule,
    args: Vec<ir::ValueId>,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    log::debug!("\t\tadd invoke: {}", inst_rule.to_string());

    // // This causes strange 101 error during test
    // if inst_rule.path.len() != 1 {
    //   return Err(format!(
    //     "unsupported inst rule {} of inst-path length > 1",
    //     inst_rule.to_string()
    //   ));
    // }
    let inst = {
      let inst = legalize_as_fir_id(inst_rule.path[0].clone());
      if &inst == "self" {
        "".to_string()
      } else {
        inst
      }
    };

    // TODO: this is ugly
    if inst_rule.rule_name.starts_with("_write_res")
      || inst_rule.rule_name.starts_with("_read_arg")
    {
      return Ok(());
    }

    let args_expr = args
      .iter()
      .map(|arg| {
        let expr = visitor.expr_by_value(*arg, data);

        self.register_expr(*arg, &expr.as_ref().unwrap(), data);
        expr
      })
      .collect::<Result<Vec<_>, anyhow::Error>>()?;

    let mut rsts_expr = Vec::new();
    for res in rsts {
      let res_expr = visitor.expr_by_value(res, data)?;
      self.register_expr(res, &res_expr, data);
      rsts_expr.push(res_expr);
    }

    // fetch the rule impl according to inst_rule
    let RuleImpl {
      args, rsts, enable, ..
    } = if let Some(rule_impl) = visitor
      .rule_impls
      .get(&data.rule_id_by_inst_rule(&inst_rule))
    {
      rule_impl
    } else {
      let module_name = data.resolve_path(&inst_rule.path);
      log::error!(
        "rule impl not found: {}::{}",
        module_name,
        inst_rule.rule_name
      );
      log::error!(
        "existing rules: {}",
        visitor
          .rule_impls
          .keys()
          .map(|k| k.to_string())
          .collect::<Vec<_>>()
          .join(",")
      );
      return Err(
        data
          .report_error(format!(
            "rule impl not found: {}::{}",
            module_name, inst_rule.rule_name
          ))
          .into(),
      );
    };

    // connect args and rsts
    for (arg, arg_expr) in args.iter().zip(args_expr) {
      self.connect(arg.clone().prefix_with(&inst, "."), arg_expr);
    }
    for (res, res_expr) in rsts.iter().zip(rsts_expr) {
      self.connect(res_expr, res.clone().prefix_with(&inst, "."));
    }
    // add enable
    if let Some(enable) = enable {
      self.connect(
        enable.clone().prefix_with(&inst, "."),
        fir::Expression::ones(1),
      );
    }

    Ok(())
  }

  pub fn add_field(
    &mut self,
    res: ir::ValueId,
    value: ir::ValueId,
    field: ir::Field,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    match field {
      ir::Field::Name(name) => {
        let res_expr = visitor.expr_by_value(res, data)?;
        self.register_expr(res, &res_expr, data);
        let struct_expr = visitor.expr_by_value(value, data)?;
        self.connect(res_expr, fir::Expression::sub_field(struct_expr, name));
        Ok(())
      }
      ir::Field::Index(idx) => {
        let res_expr = visitor.expr_by_value(res, data)?;
        self.register_expr(res, &res_expr, data);
        let struct_expr = visitor.expr_by_value(value, data)?;
        self.connect(
          res_expr,
          fir::Expression::sub_index(struct_expr, idx as usize),
        );
        Ok(())
      }
    }
  }

  #[allow(unused)]
  pub fn add_aggregate(
    &mut self,
    res: ir::ValueId,
    values: Vec<ir::ValueId>,
    fields: Vec<ir::Field>,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let all_named = fields.iter().all(|f| matches!(f, ir::Field::Name(_)));
    if all_named {
      let field_names = fields
        .iter()
        .map(|f| match f {
          ir::Field::Name(name) => name.clone(),
          _ => unreachable!(),
        })
        .collect::<Vec<_>>();
      let res_expr = visitor.expr_by_value(res, data)?;
      self.register_expr(res, &res_expr, data);
      let values_expr = values
        .iter()
        .map(|v| visitor.expr_by_value(*v, data))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

      // we need to connect each field to the corresponding value
      for (field, value) in field_names.into_iter().zip(values_expr) {
        self
          .connect(fir::Expression::sub_field(res_expr.clone(), field), value);
      }
    } else {
      let all_indexed = fields.iter().all(|f| matches!(f, ir::Field::Index(_)));
      if all_indexed {
        let res_expr = visitor.expr_by_value(res, data)?;
        self.register_expr(res, &res_expr, data);
        let values_expr = values
          .iter()
          .map(|v| visitor.expr_by_value(*v, data))
          .collect::<Result<Vec<_>, anyhow::Error>>()?;

        let field_indices = fields
          .iter()
          .map(|f| match f {
            ir::Field::Index(idx) => *idx,
            _ => unreachable!(),
          })
          .collect::<Vec<_>>();
        // we need to connect each index to the corresponding value
        for (idx, value) in field_indices.into_iter().zip(values_expr) {
          self.connect(
            fir::Expression::sub_index(res_expr.clone(), idx as usize),
            value,
          );
        }
      } else {
        return Err(
          data
            .report_error(format!(
              "unsupported aggregate (neither all-named nor all-indexed): {:?}",
              fields
            ))
            .into(),
        );
      }
    }
    Ok(())
  }

  pub fn remove_duplicate_vars(&mut self, body: &mut Vec<Statement>) {
    let mut var_set = HashSet::new();
    for var in self.vars.iter() {
      var_set.insert(var.to_string());
    }

    body.retain(|v| !var_set.contains(&v.to_string()));
  }

  pub fn push_when(
    &mut self,
    rsts: Vec<ir::ValueId>,
    cond: ir::ValueId,
    mut then_body: FirStmtBuilder,
    mut else_body: Option<FirStmtBuilder>,
    visitor: &mut transform::ToFirVisitor,
    data: &mut passes::VisitorData,
  ) -> Result<(), anyhow::Error> {
    let cond_expr = visitor.expr_by_value(cond, data)?;
    let mut rsts_expr = vec![];
    for res in rsts {
      let res_expr = visitor.expr_by_value(res, data)?;
      self.register_expr(res, &res_expr, data);
      rsts_expr.push(res_expr);
    }
    then_body.assign_to(rsts_expr.clone());
    else_body.as_mut().map(|b| b.assign_to(rsts_expr));
    let mut then_body = then_body.into_inner();
    self.remove_duplicate_vars(&mut then_body);
    let else_body = else_body.map(|b| {
      let mut b = b.into_inner();
      self.remove_duplicate_vars(&mut b);
      b
    });

    self.push_stmt(fir::Statement::when(
      cond_expr,
      then_body.into(),
      else_body.map_or(fir::Statement::EmptyStmt, |b| b.into()),
    ));
    Ok(())
  }
}
