use super::*;
use serde::de::Expected;
use std::cmp::max;
use std::env;

pub struct LegalCheckPass {}

pub fn prim_type_check(
  prim: &ir::Prim,
  input_types: Vec<ir::Type>,
  attrs: &[u32],
) -> Result<ir::Type, anyhow::Error> {
  // pre-check
  match prim.clone() {
    ir::Prim::Add
    | ir::Prim::Sub
    | ir::Prim::Or
    | ir::Prim::Xor
    | ir::Prim::And => {
      if input_types.len() != 2 {
        anyhow::bail!(format!(
          "{} takes 2 inputs, got {}.",
          prim.ir_dump(),
          input_types.len(),
        ));
      }
    }
    ir::Prim::Not => {
      if input_types.len() != 1 {
        anyhow::bail!(format!(
          "{} takes 1 input, got {}.",
          prim.ir_dump(),
          input_types.len(),
        ));
      }
    }
    ir::Prim::Shl => {
      if input_types.len() != 1 {
        anyhow::bail!(format!(
          "{} takes 2 inputs, got {}.",
          prim.ir_dump(),
          input_types.len(),
        ));
      }
    }
    _ => {
      anyhow::bail!(format!("{} is not a supported prim op", prim.ir_dump()))
    }
  }
  match prim {
    ir::Prim::Add | ir::Prim::Sub => match input_types[0] {
      ir::Type::UInt(width_first) => match input_types[1] {
        ir::Type::UInt(width_second) => {
          Ok(ir::Type::UInt(max(width_first, width_second) + 1 as u32))
        }
        _ => anyhow::bail!("type not same."),
      },
      _ => anyhow::bail!("not implemented."),
    },
    ir::Prim::And | ir::Prim::Or | ir::Prim::Xor => match input_types[0] {
      ir::Type::UInt(width_first) => match input_types[1] {
        ir::Type::UInt(width_second) => {
          Ok(ir::Type::UInt(max(width_first, width_second)))
        }
        _ => anyhow::bail!("type not same."),
      },
      _ => anyhow::bail!("not implemented."),
    },
    ir::Prim::Not => Ok(input_types[0].clone()),
    ir::Prim::Shl => {
      let shift_width = attrs[0] as usize;
      match input_types[0] {
        ir::Type::UInt(width) => {
          let new_width = width + shift_width as u32;
          Ok(ir::Type::UInt(new_width))
        }
        _ => anyhow::bail!("type not same."),
      }
    }
    _ => anyhow::bail!("not implemented."),
  }
}

pub fn invoke_legal_check(
  op: &ir::Op,
  data: &VisitorData,
) -> anyhow::Result<()> {
  match op.inner() {
    ir::OpEnum::Invoke(ir::InvokeOp {
      inst_rule,
      args,
      res,
    }) => {
      let rule_name = inst_rule.rule_name.clone();
      let module_name = data.resolve_path(&inst_rule.path);
      {
        let module =
          data.circuit.module(&module_name).ok_or(data.report_error(
            format!("Invoke error, module {} not found.", module_name),
          ))?;
        let rule = if data.module.name() == module_name {
          data.rule_backup.iter().find(|r| r.name == rule_name)
        } else {
          module.rules().find(|r| r.name == rule_name)
        };
        if let Some(rule) = rule {
          if rule.is_always() {
            anyhow::bail!(data.report_error(format!(
              "Invoke error, always rule {} cannot be invoked.",
              rule_name
            )));
          }

          for (idx, (arg, rule_input)) in
            args.iter().zip(rule.inputs()).enumerate()
          {
            let arg_ty = data.type_of(*arg).unwrap();
            let rule_input_ty =
              data.type_of_in_module(rule_input, module).unwrap();
            if arg_ty != rule_input_ty {
              match (arg_ty, rule_input_ty) {
                (ir::Type::SInt(_), ir::Type::SInt(_))
                | (ir::Type::UInt(_), ir::Type::UInt(_)) => {}
                (provided, expected) => {
                  anyhow::bail!(data.report_error_at_op(format!(
                    "type mismatch, expected {} but got {}, for invoke op {}'s {}-th input.",
                    expected.ir_dump(),
                    provided.ir_dump(),
                    op.ir_dump_with(&data.module.values),
                    idx,
                  ), op));
                }
              }
            }
          }

          for (idx, (res, rule_output)) in
            res.iter().zip(rule.outputs()).enumerate()
          {
            let res_ty = data.type_of(*res).unwrap();
            let rule_output_ty =
              data.type_of_in_module(rule_output, module).unwrap();
            if res_ty != rule_output_ty {
              match (res_ty, rule_output_ty) {
                (ir::Type::SInt(_), ir::Type::SInt(_))
                | (ir::Type::UInt(_), ir::Type::UInt(_)) => {}
                (expected, provided) => {
                  anyhow::bail!(data.report_error_at_op(format!(
                    "type mismatch, expected {} but got {}, for invoke op {}'s {}-th output.",
                    expected.ir_dump(),
                    provided.ir_dump(),
                    op.ir_dump_with(&data.module.values),
                    idx,
                  ), op));
                }
              }
            }
          }
          return Ok(());
        } else {
          anyhow::bail!(data.report_error(format!(
            "Invoke error, rule {} not found in module {}.",
            rule_name, module_name
          )));
        }
      }
    }
    _ => {}
  }
  Ok(())
}

pub fn op_guard_check(op: &ir::Op, data: &VisitorData) -> anyhow::Result<()> {
  if let ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) = op.inner() {
    let module_name = data.resolve_path(&inst_rule.path);
    let rule_name = inst_rule.rule_name.clone();
    let module = data
      .circuit
      .module(&module_name)
      .ok_or(data.report_error(format!("module {} not found.", module_name)))?;
    let rule =
      module
        .rules()
        .find(|r| r.name == rule_name)
        .ok_or(data.report_error(format!(
          "rule {} not found in module {}.",
          rule_name, module_name
        )))?;
    if rule.has_side_effect() {
      anyhow::bail!(data.report_error(format!(
        "rule {} which has side effect, should not used in guard.",
        rule_name
      )));
    }
  }
  Ok(())
}

pub fn op_type_check(
  op: &ir::Op,
  input_types: Vec<ir::Type>,
  data: &VisitorData,
) -> anyhow::Result<()> {
  let expected_output_types = op_infer_type(op, input_types, data)?;

  let actual_output_types: Vec<ir::Type> =
    op.outputs().map(|id| data.type_of(id).unwrap()).collect();

  if expected_output_types.len() != actual_output_types.len() {
    anyhow::bail!(data.report_error(format!(
      "type check failed, expected {} outputs, got {} for op {}.",
      expected_output_types.len(),
      actual_output_types.len(),
      op.ir_dump_with(&data.module.values),
    )));
  }

  for (idx, (expected, actual)) in expected_output_types
    .iter()
    .zip(actual_output_types.iter())
    .enumerate()
  {
    if expected != actual {
      match (expected, actual) {
        (ir::Type::SInt(_), ir::Type::SInt(_))
        | (ir::Type::UInt(_), ir::Type::UInt(_)) => {}
        _ => {
          anyhow::bail!(data.report_error(format!(
            "type mismatch, expected {} but got {}, for op {}'s {}-th output.",
            expected.ir_dump(),
            actual.ir_dump(),
            op.ir_dump_with(&data.module.values),
            idx,
          )));
        }
      }
    }
  }

  Ok(())
}

impl LegalCheckPass {
  pub fn new() -> Self {
    Self {}
  }

  fn legal_check(
    &mut self,
    op: &ir::Op,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    invoke_legal_check(op, data)
  }

  fn guard_check(
    &mut self,
    op: &ir::Op,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    op_guard_check(op, data)
  }

  fn type_check(
    &mut self,
    op: &ir::Op,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    let mut input_types = vec![];
    for input in op.inputs() {
      if let Some(itype) = data.type_of(input) {
        input_types.push(itype);
      } else {
        anyhow::bail!(data.report_error(format!(
          "type of {} as an input of op {} is none!",
          input,
          op.ir_dump_with(&data.module.values),
        )));
      }
    }
    op_type_check(op, input_types, data)?;
    Ok(())
  }

  fn return_value_check(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    if let Some(rule) = &data.rule {
      if rule.is_ext() {
        return Ok(());
      }
      if rule
        .ops()
        .last()
        .map(|last_op| last_op.num_outputs())
        .unwrap_or(0)
        != rule.outputs().len()
      {
        anyhow::bail!(data.report_error_at_span(
          format!(
          "return value check failed, expected {} outputs, got {} for rule {}.",
          rule.outputs().len(),
          rule.ops().last().map(|last_op| last_op.num_outputs()).unwrap_or(0),
          rule.name(),
        ),
          rule
            .ops()
            .last()
            .map(|last_op| last_op.span())
            .unwrap_or(None)
        ));
      }

      if rule.outputs().len() > 0 {
        let last_op = rule.ops().last().unwrap();
        for (idx, (res, rule_output)) in
          last_op.outputs().zip(rule.outputs()).enumerate()
        {
          let res_ty = data.type_of(res).unwrap();
          let rule_output_ty = data.type_of(rule_output).unwrap();
          if res_ty != rule_output_ty {
            match (res_ty, rule_output_ty) {
              (ir::Type::SInt(_), ir::Type::SInt(_))
              | (ir::Type::UInt(_), ir::Type::UInt(_)) => {}
              (provided, expected) => {
                anyhow::bail!(data.report_error_at_op(
              format!(
                "type mismatch, expected {} but got {}, for rule {}'s {}-th output.",
                expected.ir_dump(),
                provided.ir_dump(),
                rule.name(),
                idx,
                  ),
                  last_op
                ));
              }
            }
          }
        }
      }
    }
    Ok(())
  }
}

impl Visitor for LegalCheckPass {
  fn name() -> &'static str {
    "LegalCheckPass"
  }

  /// tb module should have its own legal check pass
  fn skip_tb() -> bool {
    true
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    let guard_ops = data
      .rule()
      .guard()
      .map(|op: &crate::Op| op.clone())
      .collect::<Vec<_>>();
    for op in guard_ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        self.type_check(&op, data)?;
        self.legal_check(&op, data)?;
        self.guard_check(&op, data)?;
      }
    }

    let ops = data.rule().ops().map(|op| op.clone()).collect::<Vec<_>>();
    for op in ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        self.type_check(&op, data)?;
        self.legal_check(&op, data)?;
      }
    }

    self.return_value_check(data)?;

    // keep the rule as is
    Ok((vec![data.take_rule()], vec![]))
  }
}
