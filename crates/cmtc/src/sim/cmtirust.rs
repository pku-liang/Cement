#![allow(dead_code)]
#![allow(unused)]

use super::*;
use cmtc::passes::*;
use std::collections::{HashMap, VecDeque};

trait ToRust {
  fn to_rust(&self) -> String;
}

impl ToRust for ir::Cmp {
  fn to_rust(&self) -> String {
    match self {
      ir::Cmp::Eq => "==".to_string(),
      ir::Cmp::Neq => "!=".to_string(),
      ir::Cmp::Lt => "<".to_string(),
      ir::Cmp::Leq => "<=".to_string(),
      ir::Cmp::Gt => ">".to_string(),
      ir::Cmp::Geq => ">=".to_string(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrintItem {
  Raw(String),
  Signal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RustExpr {
  None,
  //  auto res  = rhs
  Assign(String, String),
  //  type     res  = lit_str
  Lit(String, String, String),
  //  res   =   lhs    cmp    rhs
  Cmp(String, String, String, String),
  //  res   =   prim   (inputs  ,      attrs)
  Prim(String, ir::Prim, Vec<String>, Vec<String>),
  //  res   =   invoke  full_path(inputs)
  Invoke(Vec<String>, RuleInfo, Vec<String>),
  // block {}
  Block(Vec<RustExpr>),
  // res = if (cond) { body } else { }
  If(Vec<String>, String, Box<RustExpr>, Box<Option<RustExpr>>),
  // return res
  Return(Vec<String>),
  SimExit,
  Println(Vec<PrintItem>),
}

impl ToRust for ir::Prim {
  fn to_rust(&self) -> String {
    match self {
      ir::Prim::Add => "+".to_string(),
      ir::Prim::Sub => "-".to_string(),
      ir::Prim::Mul => "*".to_string(),
      ir::Prim::Div => "/".to_string(),
      ir::Prim::Rem => "%".to_string(),
      ir::Prim::And => "&".to_string(),
      ir::Prim::Or => "|".to_string(),
      ir::Prim::Xor => "^".to_string(),
      ir::Prim::Shl => "<<".to_string(),
      ir::Prim::Shr => ">>".to_string(),
      ir::Prim::DShl => "<<".to_string(),
      ir::Prim::DShr => ">>".to_string(),
      ir::Prim::Not => "~".to_string(),
      ir::Prim::Neg => "-".to_string(),
      _ => unimplemented!(),
    }
  }
}

fn prim_to_rust(
  prim: &ir::Prim,
  res: &str,
  inputs: &Vec<String>,
  attrs: &Vec<String>,
) -> String {
  // format!{"/*prim*/"}
  match prim {
    ir::Prim::Add
    | ir::Prim::Sub
    | ir::Prim::Mul
    | ir::Prim::Div
    | ir::Prim::Rem
    | ir::Prim::And
    | ir::Prim::Or
    | ir::Prim::Xor => {
      format!(
        "let {} = {}.clone() {} {}.clone();",
        res,
        inputs[0],
        prim.to_rust(),
        inputs[1]
      )
    }
    ir::Prim::Neg | ir::Prim::Not => {
      format!("let {}.clone() = {} {}.clone();", res, prim.to_rust(), inputs[0])
    }
    ir::Prim::Shl | ir::Prim::Shr => {
      format!(
        "let {} = {}.clone() {} {}.clone();",
        res,
        inputs[0],
        prim.to_rust(),
        attrs[0]
      )
    }
    ir::Prim::DShl | ir::Prim::DShr => {
      format!(
        "let {} = {}.clone() {} {}.clone();",
        res,
        inputs[0],
        prim.to_rust(),
        attrs[0]
      )
    }
    _ => {
      format!("/*prim*/")
    }
  }
}

fn invoke_to_rust(
  res: &Vec<String>,
  rule_info: &RuleInfo,
  args: &Vec<String>,
) -> String {
  if rule_info.rule_name.starts_with("_"){
    return "".to_string();
  }

  let mut invoke_args = args.clone();
  for invoke in invoke_args.iter_mut(){
    invoke.push_str(".clone()");
  }

  let schedule_call = format!(
    "sim_server.schedule_method( RuleInfo::new_method_rule( \"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string()), vec![{}]);",
    rule_info.full_path,
    rule_info.module_name,  
    rule_info.rule_name,
    invoke_args.join(", ")
  );

  let result = if res.is_empty() {
    schedule_call
  } else {
    let res_str = format!("vec_{}", res.join("_"));
    let mut code_str = format!("let {} = {}", res_str, schedule_call);
    for (i,res_var) in res.iter().enumerate(){
      code_str.push_str(&format!("\n\tlet {} = {}[{}].clone();", res_var, res_str, i));
    }
    code_str
  };

  return result;
  // FIXME: here is uncomplete
}

fn if_to_rust(
  res: &Vec<String>,
  cond: &str,
  then_body: &Box<RustExpr>,
  else_body: &Box<Option<RustExpr>>,
) -> String {
  let _s = format!("{} = if ({}) {{ {} }} else {{ {} }}", res.join(", "), cond, then_body.to_rust(), (*else_body).clone().map(|e| e.to_rust()).unwrap_or("".to_string()));
  _s
}

impl ToRust for RustExpr {
  fn to_rust(&self) -> String {
    match self {
      RustExpr::None => "/* none */".to_string(),
      RustExpr::Assign(res, value) => format!("let {} = {}.clone();", res, value),
      RustExpr::Lit(ty, res, lit) => format!("{} {} = {};", ty, res, lit),
      RustExpr::Cmp(res, a, cmp, b) => {
        format!("let {} = {}.clone() {} {}.clone();", res, a, cmp, b)
      }
      RustExpr::Prim(res, prim, inputs, attrs) => {
        prim_to_rust(prim, res, inputs, attrs)
      }
      RustExpr::Invoke(res, full_path, args) => {
        invoke_to_rust(res, full_path, args)
      }
      RustExpr::Block(vec) => {
        format!(
          "{}",
          vec
            .iter()
            .map(|e| e.to_rust())
            .collect::<Vec<_>>()
            .join("\n\t")
        )
      }
      RustExpr::If(res, cond, then_body, else_body) => {
        if_to_rust(res, cond, then_body, else_body)
      }
      RustExpr::Return(res) => {
        format!("return vec![{}];", res.join(", "))
      }
      RustExpr::SimExit => {
        format!("process::exit(0);\n")
      }
      RustExpr::Println(items) => {
        let mut s = String::new();
        for item in items.iter() {
          match item {
            PrintItem::Raw(raw) => {
              s.push_str(&format!("print!(\"{}\");\n", raw))
            }
            PrintItem::Signal(signal) => {
              s.push_str(&format!("print!(\"{{}}\",format!(\"{{:?}}\",{}));\n", signal))
            }
          }
        }
        s.push_str("print!(\"\\n\");\n");
        s
      }
    }
  }
}

impl RustExpr {
  pub fn eval(
    op: ir::Op,
    circuit: &ir::Circuit,
    cur_module_run: &ModuleRun,
    full_path: &str,
    belonged_module: &str,
  ) -> anyhow::Result<Self> {
    let module = circuit.module(&belonged_module).unwrap();
    let rust_expr = match op.inner() {
      ir::OpEnum::Nop(_) => RustExpr::None,
      ir::OpEnum::Assign(AssignOp { res, value }) => {
        RustExpr::Assign(module.value_to_rust(*res), module.value_to_rust(*value))
      }
      ir::OpEnum::Lit(LitOp {
        value: TypedLit { lit, ty: _ },
        res,
      }) => RustExpr::Lit(
        "let".to_string(),
        module.value_to_rust(*res),
        format!("BigInt::from({})", lit.to_demical()),
      ),
      ir::OpEnum::Cmp(CmpOp { res, a, b, cmp }) => RustExpr::Cmp(
        module.value_to_rust(*res),
        module.value_to_rust(*a),
        cmp.to_rust(),
        module.value_to_rust(*b),
      ),
      ir::OpEnum::Prim(PrimOp {
        prim,
        res,
        inputs,
        attrs,
      }) => RustExpr::Prim(
        module.value_to_rust(*res),
        *prim,
        inputs.iter().map(|i| module.value_to_rust(*i)).collect(),
        attrs.iter().map(|a| format!("{}", a)).collect(),
      ),
      ir::OpEnum::Invoke(InvokeOp {
        res,
        inst_rule,
        args,
      }) => RustExpr::Invoke(
        res.iter().map(|r| module.value_to_rust(*r)).collect(),
        create_rule_info(
          &inst_rule,
          &belonged_module.to_string(),
          &cur_module_run,
          &full_path.to_string(),
        ),
        args.iter().map(|a| module.value_to_rust(*a)).collect(),
      ),
      ir::OpEnum::Block(body) => RustExpr::Block(
        (*body)
          .ops
          .iter()
          .map(|o| {
            RustExpr::eval(
              o.clone(),
              circuit,
              cur_module_run,
              &full_path,
              &belonged_module,
            )
          })
          .collect::<Result<Vec<_>, _>>()?,
      ),
      ir::OpEnum::If(ifop) => {
        let IfOp {
          res,
          cond,
          then_body,
          else_body,
        } = (**ifop).clone();
        RustExpr::If(
          res.iter().map(|r| module.value_to_rust(*r)).collect(),
          module.value_to_rust(cond),
          Box::new(RustExpr::eval(
            then_body.clone(),
            circuit,
            cur_module_run,
            &full_path,
            &belonged_module,
          )?),
          if let Some(else_body) = else_body {
            Box::new(Some(RustExpr::eval(
              else_body.clone(),
              circuit,
              cur_module_run,
              &full_path,
              &belonged_module,
            )?))
          } else {
            Box::new(None)
          },
        )
      }
      ir::OpEnum::Return(ReturnOp { values }) => RustExpr::Return(
        values.iter().map(|v| module.value_to_rust(*v)).collect(),
      ),
      ir::OpEnum::Call(_) => unimplemented!(),
      ir::OpEnum::Field(_) => unimplemented!(),
      ir::OpEnum::Aggregate(_) => unimplemented!(),
      ir::OpEnum::Timed(_, _) => unimplemented!(),
      ir::OpEnum::Delay(_) => unimplemented!(),
      ir::OpEnum::DynDelay(_) => unimplemented!(),
      ir::OpEnum::Step(_) => unimplemented!(),
      ir::OpEnum::Seq(_) => unimplemented!(),
      ir::OpEnum::Par(_) => unimplemented!(),
      ir::OpEnum::Branch(_) => unimplemented!(),
      ir::OpEnum::For(_) => unimplemented!(),
      ir::OpEnum::SimPrint(SimPrintOp { items }) => RustExpr::Println(
        items.into_iter().map(|i| match i {
            cmtir::PrintItem::String(s) => PrintItem::Raw(s.clone()),
            cmtir::PrintItem::Value(value_id) => PrintItem::Signal(module.value_to_rust(*value_id)),
        }).collect()
      ),
      ir::OpEnum::SimExit(_) => RustExpr::SimExit,
      ir::OpEnum::SimFromInt(_) => unimplemented!(), 
    };
    Ok(rust_expr)
  }
}

fn concat_path(path: &str, name: &str) -> String {
  format!("{}{}{}", path, if path.is_empty() { "" } else { "_" }, name)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ImplicitGuard {
  Run(RuleInfo),
  NotRun(RuleInfo),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RustRuleRun {
  operations: Vec<RustExpr>,
  full_path: String,
  module: String,
  rule_name: String,
  is_private: bool,
  is_always: bool,
  explicit_guards: Vec<RustExpr>,
  implicit_guards: Vec<ImplicitGuard>,
  inputs: Vec<String>,
  outputs: Vec<String>,
}

impl RustRuleRun {
  pub fn create(
    rule_run: interface::RuleRun,
    belonged_module: String,
    cur_module_run: &ModuleRun,
    full_path: String,
    rule_name: String,
    circuit: &ir::Circuit,
  ) -> anyhow::Result<Self> {
    let mut operations: Vec<RustExpr> = vec![];
    for op in rule_run.ops() {
      operations.push(RustExpr::eval(
        op.clone(),
        circuit,
        cur_module_run,
        &full_path,
        &belonged_module,
      )?);
    }
    if let Some(last_op) = rule_run.ops().last(){
      if !matches!(last_op.inner(), ir::OpEnum::Return(_)){
        operations.push(RustExpr::eval(
          ir::OpEnum::Return(ir::ReturnOp{values:vec![]}).into(),
          circuit,
          cur_module_run,
          &full_path,
          &belonged_module,
        )?);
      }
    }

    let mut explicit_guards = vec![];
    let mut implicit_guards = vec![];

    for guard_op in rule_run.guards() {
      match guard_op {
        interface::Guard::Explicit(guard_op) => {
          if let ir::OpEnum::Block(body_op) = guard_op.clone().inner {
            if !body_op.ops.is_empty() {
              explicit_guards.push(RustExpr::eval(
                guard_op.clone(),
                circuit,
                cur_module_run,
                &full_path,
                &belonged_module,
              )?);
            }
          }
        }
        interface::Guard::Implicit(status) => match status {
          interface::RunStatus::Run(inst_rule) => {
            if !inst_rule.rule_name.starts_with("_")
            {
              let rule_info = create_rule_info(
                &inst_rule,
                &belonged_module,
                &cur_module_run,
                &full_path,
              );
              implicit_guards.push(ImplicitGuard::Run(rule_info));
            }
          }
          interface::RunStatus::NotRun(inst_rule) => {
            if !inst_rule.rule_name.starts_with("_")
            {
              let rule_info = create_rule_info(
                &inst_rule,
                &belonged_module,
                &cur_module_run,
                &full_path,
              );
              implicit_guards.push(ImplicitGuard::NotRun(rule_info));
            }
          }
        },
        _ => {}
      }
    }

    let module = circuit.module(&belonged_module).unwrap();
    Ok(Self {
      operations,
      full_path,
      module: belonged_module,
      rule_name,
      is_always: rule_run.is_always(),
      is_private: rule_run.is_private(),
      explicit_guards,
      implicit_guards,
      inputs: rule_run
        .get_inputs()
        .iter()
        .map(|id| module.value_to_rust(*id))
        .collect(),
      outputs: rule_run
        .get_outputs()
        .iter()
        .map(|id| module.value_to_rust(*id))
        .collect(),
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleInfo {
  full_path: String,
  module_name: String,
  rule_name: String,
}

pub struct Cmtirust {
  circuit: ir::Circuit,
  sim_data: interface::SimData,
  rules: Vec<RustRuleRun>,
  rule_top: HashMap<RuleInfo, Vec<RuleInfo>>,
  regs: Vec<String>,
  wires: Vec<String>,
  integers: Vec<String>,
}

fn get_rules_name_in_topo_order(module: &mut ir::Module) -> Vec<String> {
  let mut rules: Vec<Rule> = std::mem::take(module.rules_mut());

  let ordered_rules =
    <EmptyVisitor as Visitor>::topo_sort_rules(rules.iter(), |i, r| {
      (r.name().to_string(), i)
    });
  ordered_rules
    .into_iter()
    .map(|i| rules[i].name.clone())
    .collect()
}

fn create_rule_info(
  inst_rule: &InstRule,
  cur_module_name: &String,
  cur_module_run: &ModuleRun,
  cur_path: &String,
) -> RuleInfo {
  let module_name =
    if inst_rule.path.is_empty() || inst_rule.path.last().unwrap() == "self" {
      cur_module_name.clone()
    } else {
      cur_module_run
        .instances()
        .iter()
        .find(|inst| inst.name == *inst_rule.path.last().unwrap())
        .unwrap()
        .clone()
        .module
    };

  let rule_info =
    if inst_rule.path.is_empty() || inst_rule.path.last().unwrap() == "self" {
      RuleInfo {
        full_path: cur_path.clone(),
        module_name: module_name,
        rule_name: inst_rule.rule_name.clone(),
      }
    } else {
      RuleInfo {
        full_path: concat_path(cur_path, inst_rule.path.last().unwrap()),
        module_name: module_name,
        rule_name: inst_rule.rule_name.clone(),
      }
    };

  rule_info
}

impl Cmtirust {
  pub fn new(circuit: ir::Circuit, sim_data: interface::SimData) -> Self {
    Self {
      circuit,
      sim_data,
      rules: vec![],
      rule_top: HashMap::new(),
      regs: vec![],
      wires: vec![],
      integers: vec![]
    }
  }

  fn init(&mut self) -> anyhow::Result<()> {
    println!("Iterating over all modules");
    let mut fifo = VecDeque::new();

    let top_module_name = self.circuit.top_module_name.clone();
    println!("\tTop module name: {}", top_module_name);

    fifo.push_back((top_module_name, "".to_string()));

    while !fifo.is_empty() {
      let (cur_module_name, cur_path) = fifo.pop_front().unwrap();
      //ignore wire
      if cur_module_name.starts_with("wire_")
      {
        self.wires.push(cur_path.clone());
      } else if cur_module_name.starts_with("reg_") {
        self.regs.push(cur_path.clone());
      }else if cur_module_name == "integer"{
        self.integers.push(cur_path.clone());
      }

      println!(
        "\tCurrent module name: {}, path: {}",
        cur_module_name, cur_path
      );

      let cur_module_run: &ModuleRun = self
        .sim_data
        .module_run(&cur_module_name)
        .ok_or(anyhow::anyhow!("Module {} not found", cur_module_name))?;

      let mut module = self.circuit.module(&cur_module_name).unwrap().clone();
      let rule_names = get_rules_name_in_topo_order(&mut module);

      for rule_name in rule_names.iter().rev() {
        if rule_name.starts_with("_"){
          continue;
        }

        let rule_run = cur_module_run
          .rule_run(&rule_name)
          .ok_or(anyhow::anyhow!("Rule {} not found", rule_name))?
          .clone();

        for op in rule_run.ops() {
          let flatten_op = <EmptyVisitor as Visitor>::flatten_op(op);
          for op in flatten_op.into_iter() {
            if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
              let rule_info = create_rule_info(
                &inst_rule,
                &cur_module_name,
                &cur_module_run,
                &cur_path,
              );
              let cur_rule_info = RuleInfo {
                full_path: cur_path.clone(),
                module_name: cur_module_name.clone(),
                rule_name: rule_name.clone(),
              };
              if rule_run.is_always() {
                if !cur_rule_info.rule_name.starts_with("_"){
                  self
                  .rule_top
                  .entry(rule_info.clone())
                  .or_insert_with(Vec::new)
                  .push(cur_rule_info.clone());
                }
              } else {
                if let Some(top_rule_info) = self.rule_top.get(&cur_rule_info) {
                  self
                    .rule_top
                    .insert(rule_info.clone(), top_rule_info.clone());
                }
              }
            }
          }
        }

        self.rules.push(RustRuleRun::create(
          rule_run.clone(),
          cur_module_name.to_string(),
          &cur_module_run,
          cur_path.clone(),
          rule_name.clone(),
          &self.circuit,
        )?);
      }

      for InstDef { name, module, .. } in cur_module_run.instances() {
        fifo.push_back((module.clone(), concat_path(&cur_path, name)));
      }
    }

    Ok(())
  }

  fn find_rule_run(&self, rule_info: &RuleInfo) -> &RustRuleRun {
    self.rules.iter()
        .find(|rule_run| 
            rule_run.rule_name == rule_info.rule_name &&
            rule_run.module == rule_info.module_name &&
            rule_info.full_path == rule_run.full_path
        ).unwrap()
  }

  fn get_init_data_structure_code(&self) -> Vec<String> {
    let mut init_code = vec![];
    init_code.push("pub fn init_data_structure(&mut self) {".to_string());
    for rule_run in self.rules.iter() {
      init_code.push(format!(
        "self.sim_server.add_rule(\"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string(), {}, {});",
        &rule_run.full_path,
        &rule_run.module,
        &rule_run.rule_name,
        &rule_run.is_always,
        &rule_run.is_private
      ));
      init_code.push(format!(
        "self.sim_server.add_function(SimWhole::{}_{}_{});",
        &rule_run.module, &rule_run.full_path, &rule_run.rule_name
      ));
    }
    for reg in self.regs.iter() {
      init_code.push(format!(
        "self.sim_server.add_reg(\"{}\".to_string());",
        &reg
      ));
    }

    for wire in self.wires.iter() {
      init_code.push(format!(
        "self.sim_server.add_wire(\"{}\".to_string());",
        &wire
      ));
    }

    for integer in self.integers.iter() {
      init_code.push(format!(
        "self.sim_server.add_integer(\"{}\".to_string());",
        &integer
      ));
    }

    for (method_rule, top_rules) in &self.rule_top {
      for top_rule in top_rules {
        if top_rule.rule_name.starts_with("_") || method_rule.rule_name.starts_with("_"){
          continue;
        }
        init_code.push(format!(
          "self.sim_server.add_method_top_rule(\n    self.sim_server.find_rule_id(&RuleInfo::new_method_rule(\"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string())),\n    self.sim_server.find_rule_id(&RuleInfo::new_always_rule(\"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string())),\n);\n",
          &method_rule.full_path,
          &method_rule.module_name,
          &method_rule.rule_name,
          &top_rule.full_path,
          &top_rule.module_name,
          &top_rule.rule_name,
        ));
      }
    }

    for rule_run in self.rules.iter() {
      let mut guards_str: Vec<String> = vec![];
      if !rule_run.explicit_guards.is_empty() {
        guards_str.push(format!(
          "Guard::Explicit(SimWhole::guard_{}_{}_{})",
          &rule_run.module, &rule_run.full_path, &rule_run.rule_name
        ));
      }
      for guard in rule_run.implicit_guards.iter() {
        match guard.clone() {
          ImplicitGuard::Run(rule_info) => {
            let rule_run = self.find_rule_run(&rule_info);
            let pattern = if rule_run.is_always {
              "always".to_string()
            } else {
              "method".to_string()
            };
            guards_str.push(format!(
                "Guard::Implicit(RunStatus::Run(self.sim_server.find_rule_id(&RuleInfo::new_{}_rule(\"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string()))))",
                &pattern,&rule_info.full_path, &rule_info.module_name, &rule_info.rule_name
            ));
          }
          ImplicitGuard::NotRun(rule_info) => {
            let rule_run = self.find_rule_run(&rule_info);
            let pattern = if rule_run.is_always {
              "always".to_string()
            } else {
              "method".to_string()
            };
            guards_str.push(format!(
                "Guard::Implicit(RunStatus::NotRun(self.sim_server.find_rule_id(&RuleInfo::new_{}_rule(\"{}\".to_string(), \"{}\".to_string(), \"{}\".to_string()))))",
                &pattern,&rule_info.full_path,&rule_info.module_name,  &rule_info.rule_name
            ));
          }
        }
      }
      init_code.push(format!(
        "self.sim_server.add_guard_vec(vec![{}]);",
        guards_str.join(", ")
      ));
    }
    init_code.push("}".to_string());
    init_code
  }

  fn get_method_codes(&self) -> Vec<String> {
    let mut method_codes = vec![];
    for rule_run in self.rules.iter() {
      method_codes.push(format!(
        "fn {}_{}_{}(sim_server: &mut SimServer, args: Vec<BigInt>) -> Vec<BigInt> {{",
        &rule_run.module, &rule_run.full_path, &rule_run.rule_name
      ));
      // method_codes.push(format!("\tprintln!(\"schedule {}_{}_{}\");", 
      //   &rule_run.module, &rule_run.full_path, &rule_run.rule_name));
      for (i, input) in rule_run.inputs.iter().enumerate() {
        method_codes.push(format!("\tlet {} = args[{}].clone();", input, i));
      }
      for operation in rule_run.operations.iter() {
        method_codes.push(format!("\t{}", operation.to_rust()));
      }
      if rule_run.module.starts_with("reg_"){
        if rule_run.rule_name == "read".to_string(){
          method_codes.push(format!("\tlet reg_value=sim_server.reg_value_map.get(&\"{}\".to_string()).unwrap().clone();",rule_run.full_path));
          method_codes.push("\tvec![reg_value]".to_string());
        }else{
          method_codes.push(format!("\tsim_server.reg_value_map.insert(\"{}\".to_string(), {});",rule_run.full_path,rule_run.inputs[0]));
          method_codes.push("\tvec![]".to_string());
        }
      }else if rule_run.module == "integer"{
        if rule_run.rule_name == "read".to_string(){
          method_codes.push(format!("\tlet integer_value=sim_server.integer_value_map.get(&\"{}\".to_string()).unwrap().clone();",rule_run.full_path));
          method_codes.push("\tvec![integer_value]".to_string());
        }else{
          method_codes.push(format!("\tsim_server.integer_value_map.insert(\"{}\".to_string(), {});",rule_run.full_path,rule_run.inputs[0]));
          method_codes.push("\tvec![]".to_string());
        }
      }else if rule_run.module.starts_with("wire_"){
        if rule_run.rule_name == "read".to_string(){
          method_codes.push(format!("\tlet wire_value=sim_server.wire_value_map.get(&\"{}\".to_string()).unwrap().clone();",rule_run.full_path));
          method_codes.push("\tvec![wire_value]".to_string());
        }else {
          method_codes.push(format!("\tsim_server.wire_value_map.insert(\"{}\".to_string(), {});",rule_run.full_path,rule_run.inputs[0]));
          method_codes.push("\tvec![]".to_string());
        }
      }
      method_codes.push("}".to_string());
    }
    method_codes.retain(|s| s != "\t" && s != "");
    method_codes
  }

  fn get_guard_codes(&self) -> Vec<String> {
    let mut guard_codes = vec![];
    for rule_run in self.rules.iter() {
      if !rule_run.explicit_guards.is_empty(){
        guard_codes.push(format!(
          "fn guard_{}_{}_{}(sim_server: &mut SimServer) -> bool {{",
          &rule_run.module, &rule_run.full_path, &rule_run.rule_name
        ));
        for operation in rule_run.explicit_guards.iter() {
          guard_codes.push(format!("\t{}", operation.to_rust()));
        }
        guard_codes.push("}".to_string());
      }
    }

    for string in guard_codes.iter_mut() {
      if string.contains("return vec![") && string.contains("];") {
          *string = string.replace("return vec![", "return ").replace("];", ";");
      }
    }
    guard_codes
  }

  fn get_top_module_code(&self) -> Vec<String>{
    let top_module_name = self.circuit.top_module_name.clone();
    let mut test_codes = vec![];
    test_codes.push("pub fn get_top_module() -> ModuleInfo{".to_string());
    test_codes.push(format!(
      "return ModuleInfo {{ full_path: \"\".to_string(), module_name: \"{}\".to_string() }};",
      top_module_name
    ));
    test_codes.push("}".to_string());
    test_codes
  }

  fn get_whole_sim_code(&self) -> Vec<String> {
    let mut sim_codes = vec![];
    sim_codes.append(&mut self.get_top_module_code());
    sim_codes.push("impl SimWhole {".to_string());
    sim_codes.append(& mut self.get_init_data_structure_code());
    sim_codes.append(& mut self.get_method_codes());
    sim_codes.append(& mut self.get_guard_codes());
    sim_codes.push("}".to_string());
    sim_codes
  }
}
