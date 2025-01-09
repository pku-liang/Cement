use super::*;

#[derive(Clone)]
pub enum PrintItem {
  Raw(String),
  Signal(String),
}

#[derive(Clone)]
pub enum CppExpr {
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
  Invoke(Vec<String>, String, Vec<String>),
  // block {}
  Block(Vec<CppExpr>),
  // res = if (cond) { body } else { }
  If(Vec<String>, String, Box<CppExpr>, Box<Option<CppExpr>>),
  // return res
  Return(Vec<String>),
  CanRun(String),
  HavntRun(String),
  SimExit,
  Println(Vec<PrintItem>),
  //         type    res    =rhs
  SimFromInt(String, String, String),
}

impl ToCpp for ir::Prim {
  fn to_cpp(&self) -> String {
    match self {
      ir::Prim::Add => "+".to_string(),
      ir::Prim::Sub => "-".to_string(),
      ir::Prim::Mul => "*".to_string(),
      ir::Prim::Div => "/".to_string(),
      ir::Prim::Rem => "%".to_string(),
      ir::Prim::And => "&".to_string(),
      ir::Prim::Or => "|".to_string(),
      ir::Prim::Xor => "^".to_string(),
      // ir::Prim::Andr => "~&".to_string(),
      // ir::Prim::Orr => "~|".to_string(),
      // ir::Prim::Xorr => "~^".to_string(),
      ir::Prim::Shl => "<<".to_string(),
      ir::Prim::Shr => ">>".to_string(),
      ir::Prim::DShl => "<<".to_string(),
      ir::Prim::DShr => ">>".to_string(),
      ir::Prim::Not => "~".to_string(),
      ir::Prim::Neg => "-".to_string(),
      // ir::Prim::Cat => todo!(),
      // ir::Prim::Pad => todo!(),
      // Prim::Cvt => todo!(),
      // Prim::AsUInt => todo!(),
      // Prim::AsSInt => todo!(),
      // Prim::Bits => todo!(),
      // Prim::Head => todo!(),
      // Prim::Tail => todo!(),
      _ => unimplemented!(),
    }
  }
}

fn prim_to_cpp(
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
        "auto {} = {} {} {};\n",
        res,
        inputs[0],
        prim.to_cpp(),
        inputs[1]
      )
    }
    ir::Prim::Neg | ir::Prim::Not => {
      format!("auto {} = {} {};\n", res, prim.to_cpp(), inputs[0])
    }
    ir::Prim::Shl | ir::Prim::Shr => {
      format!(
        "auto {} = {} {} {};\n",
        res,
        inputs[0],
        prim.to_cpp(),
        attrs[0]
      )
    }
    ir::Prim::DShl | ir::Prim::DShr => {
      format!(
        "auto {} = {} {} {};\n",
        res,
        inputs[0],
        prim.to_cpp(),
        attrs[0]
      )
    }
    _ => {
      format!("/*prim*/")
    }
  }
}

fn invoke_to_cpp(
  res: &Vec<String>,
  full_path: &str,
  args: &Vec<String>,
) -> String {
  let mut s = String::new();
  // DATATYPE res;
  for res in res.iter() {
    s.push_str(&format!("DATATYPE {};\n", res));
  }

  // r_<full_path>(<inputs>, <outputs>)
  s.push_str(&format!(
    "r_{}({});\n",
    full_path,
    args
      .iter()
      .chain(res.iter())
      .cloned()
      .collect::<Vec<_>>()
      .join(", ")
  ));

  s

  // let res_str = format!("vec_{}", res.join("_"));
  // format!(
  //   "auto {} = INVOKE({}, {})",
  //   res_str,
  //   full_path,
  //   args.join(", ")
  // )
  // FIXME: here is uncomplete
}

fn if_to_cpp(
  res: &Vec<String>,
  cond: &str,
  then_body: &Box<CppExpr>,
  else_body: &Box<Option<CppExpr>>,
) -> String {
  let _ = format!(
    "{} = if ({}) {{ {} }} else {{ {} }}",
    res.join(", "),
    cond,
    then_body.to_cpp(),
    else_body
      .as_ref()
      .clone()
      .map(|e| e.to_cpp())
      .unwrap_or_default()
  );
  todo!()
}

impl ToCpp for CppExpr {
  fn to_cpp(&self) -> String {
    match self {
      CppExpr::None => "/* none */".to_string(),
      CppExpr::Assign(res, value) => format!("auto {} = {};\n", res, value),
      CppExpr::Lit(ty, res, lit) => format!("{} {} = {};\n", ty, res, lit),
      CppExpr::Cmp(res, a, cmp, b) => {
        format!("bool {} = {} {} {};\n", res, a, cmp, b)
      }
      CppExpr::Prim(res, prim, inputs, attrs) => {
        prim_to_cpp(prim, res, inputs, attrs)
      }
      CppExpr::Invoke(res, full_path, args) => {
        invoke_to_cpp(res, full_path, args)
      }
      CppExpr::Block(vec) => {
        format!(
          "{}\n",
          vec
            .iter()
            .map(|e| e.to_cpp())
            .collect::<Vec<_>>()
            .join("\n")
        )
      }
      CppExpr::If(res, cond, then_body, else_body) => {
        if_to_cpp(res, cond, then_body, else_body)
      }
      CppExpr::Return(res) => {
        format!("/* return {} */\n", res.join(", "))
      }
      CppExpr::CanRun(full_path) => {
        format!("/* can run {} */\n", full_path)
      }
      CppExpr::HavntRun(full_path) => {
        format!("/* havnt run {} */\n", full_path)
      }
      CppExpr::SimExit => {
        format!("set_exit();\n")
      }
      CppExpr::Println(items) => {
        let mut s = String::new();
        for item in items.iter() {
          match item {
            PrintItem::Raw(raw) => {
              s.push_str(&format!("printf(\"{}\");\n", raw))
            }
            PrintItem::Signal(signal) => {
              s.push_str(&format!("printf(\"%lu\", {});\n", signal))
            }
          }
        }
        s.push_str("printf(\"\\n\");\n");
        s
      }
      CppExpr::SimFromInt(ty, res, rhs) => {
        format!("{ty} {res} = reinterpret_cast<{ty}>({rhs});\n")
      }
    }
  }
}

impl CppExpr {
  pub fn eval(
    op: ir::Op,
    circuit: &ir::Circuit,
    full_path: &str,
    belonged_module: &str,
  ) -> anyhow::Result<Self> {
    let module = circuit.module(&belonged_module).unwrap();
    let cpp_expr = match op.inner() {
      ir::OpEnum::Nop(_) => CppExpr::None,
      ir::OpEnum::Assign(AssignOp { res, value }) => {
        CppExpr::Assign(module.value_to_cpp(*res), module.value_to_cpp(*value))
      }
      ir::OpEnum::Lit(LitOp {
        value: TypedLit { lit, ty: _ },
        res,
      }) => CppExpr::Lit(
        "DATATYPE".to_string(),
        module.value_to_cpp(*res),
        format!("{}", lit.to_demical()),
      ),
      ir::OpEnum::Cmp(CmpOp { res, a, b, cmp }) => CppExpr::Cmp(
        module.value_to_cpp(*res),
        module.value_to_cpp(*a),
        cmp.to_cpp(),
        module.value_to_cpp(*b),
      ),
      ir::OpEnum::Prim(PrimOp {
        prim,
        res,
        inputs,
        attrs,
      }) => CppExpr::Prim(
        module.value_to_cpp(*res),
        *prim,
        inputs.iter().map(|i| module.value_to_cpp(*i)).collect(),
        attrs.iter().map(|a| format!("{}", a)).collect(),
      ),
      ir::OpEnum::Invoke(InvokeOp {
        res,
        inst_rule,
        args,
      }) => CppExpr::Invoke(
        res.iter().map(|r| module.value_to_cpp(*r)).collect(),
        concat_path(&full_path, &inst_rule.to_string_with("_")),
        args.iter().map(|a| module.value_to_cpp(*a)).collect(),
      ),
      ir::OpEnum::Block(body) => CppExpr::Block(
        (*body)
          .ops
          .iter()
          .map(|o| {
            CppExpr::eval(o.clone(), circuit, &full_path, &belonged_module)
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
        CppExpr::If(
          res.iter().map(|r| module.value_to_cpp(*r)).collect(),
          module.value_to_cpp(cond),
          Box::new(CppExpr::eval(
            then_body.clone(),
            circuit,
            &full_path,
            &belonged_module,
          )?),
          if let Some(else_body) = else_body {
            Box::new(Some(CppExpr::eval(
              else_body.clone(),
              circuit,
              &full_path,
              &belonged_module,
            )?))
          } else {
            Box::new(None)
          },
        )
      }
      ir::OpEnum::Return(ReturnOp { values }) => CppExpr::Return(
        values.iter().map(|v| module.value_to_cpp(*v)).collect(),
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
      ir::OpEnum::SimPrint(SimPrintOp { items }) => CppExpr::Println(
        items
          .into_iter()
          .map(|i| match i {
            cmtir::PrintItem::String(s) => PrintItem::Raw(s.clone()),
            cmtir::PrintItem::Value(value_id) => {
              PrintItem::Signal(module.value_to_cpp(*value_id))
            }
          })
          .collect(),
      ),
      ir::OpEnum::SimExit(_) => CppExpr::SimExit,
      ir::OpEnum::SimFromInt(SimFromInt { res, a }) => CppExpr::SimFromInt(
        "DATATYPE".to_string(),
        module.value_to_cpp(*res),
        module.value_to_cpp(*a),
      ),
    };
    Ok(cpp_expr)
  }

  pub fn can_run(
    full_path: String,
    inst_rule: ir::InstRule,
  ) -> anyhow::Result<Self> {
    Ok(Self::CanRun(concat_path(
      &full_path,
      &inst_rule.to_string_with("_"),
    )))
  }

  pub fn havnt_run(
    full_path: String,
    inst_rule: ir::InstRule,
  ) -> anyhow::Result<Self> {
    Ok(Self::HavntRun(concat_path(
      &full_path,
      &inst_rule.to_string_with("_"),
    )))
  }

  pub fn outputs(&self) -> Vec<String> {
    match self {
      CppExpr::None => vec![],
      CppExpr::Assign(res, _) => vec![res.clone()],
      CppExpr::Lit(_, res, _) => vec![res.clone()],
      CppExpr::Cmp(res, _, _, _) => vec![res.clone()],
      CppExpr::Prim(res, _, _, _) => vec![res.clone()],
      CppExpr::Invoke(res, _, _) => res.clone(),
      CppExpr::Block(vec) => {
        vec.last().map(|e| e.outputs()).unwrap_or_default()
      }
      CppExpr::If(res, _, _, _) => res.clone(),
      CppExpr::Return(res) => res.clone(),
      CppExpr::CanRun(_) => vec![],
      CppExpr::HavntRun(_) => vec![],
      CppExpr::SimExit => vec![],
      CppExpr::Println(_) => vec![],
      CppExpr::SimFromInt(_, res, _) => vec![res.clone()],
    }
  }

  pub fn one_output(&self) -> String {
    let outputs = self.outputs();
    if outputs.len() > 1 {
      panic!("expect one output, but got {}", outputs.len());
    }
    outputs.first().unwrap_or(&"true".to_string()).clone()
  }
}
