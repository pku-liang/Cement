use super::*;
use cmtir::MySpan;
use cmtir::SimFromInt;
use num::pow;
use std::cmp::{max, min};

pub fn prim_type_infer(
  prim: &ir::Prim,
  input_types: Vec<ir::Type>,
  attrs: &[u32],
  op_span: Option<MySpan>,
  data: &VisitorData,
) -> Result<ir::Type, anyhow::Error> {
  match prim {
    ir::Prim::Add | ir::Prim::Sub => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 2",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
            input_types.len()
          ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::Integer, ir::Type::Integer) => Ok(ir::Type::Integer),
        (ir::Type::UInt(width_first), ir::Type::UInt(width_second)) => {
          Ok(ir::Type::UInt(max(width_first, width_second) + 1 as u32))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(width_second)) => {
          Ok(ir::Type::SInt(max(width_first, width_second)))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Mul => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 2",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::Integer, ir::Type::Integer) => Ok(ir::Type::Integer),
        (ir::Type::UInt(width_first), ir::Type::UInt(width_second)) => {
          Ok(ir::Type::UInt(width_first + width_second))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(width_second)) => {
          Ok(ir::Type::SInt(width_first + width_second))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Div => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 2",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::Integer, ir::Type::Integer) => Ok(ir::Type::Integer),
        (ir::Type::UInt(width_first), ir::Type::UInt(_width_second)) => {
          Ok(ir::Type::UInt(width_first))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(_width_second)) => {
          Ok(ir::Type::SInt(width_first + 1))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Rem => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 2",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::Integer, ir::Type::Integer) => Ok(ir::Type::Integer),
        (ir::Type::UInt(width_first), ir::Type::UInt(width_second)) => {
          Ok(ir::Type::UInt(min(width_first, width_second)))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(width_second)) => {
          Ok(ir::Type::SInt(min(width_first, width_second)))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Pad => {
      if input_types.len() != 1 || attrs.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      let width = attrs[0];
      let input_type = input_types[0].clone();
      match input_type {
        ir::Type::UInt(width_input) => {
          Ok(ir::Type::UInt(max(width, width_input)))
        }
        ir::Type::SInt(width_input) => {
          Ok(ir::Type::SInt(max(width, width_input)))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_type.ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::AsUInt => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match input_types[0].clone() {
        ir::Type::UInt(width_input) => Ok(ir::Type::UInt(width_input)),
        ir::Type::SInt(width_input) => Ok(ir::Type::UInt(width_input)),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::AsSInt => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match input_types[0].clone() {
        ir::Type::UInt(width_input) => Ok(ir::Type::SInt(width_input)),
        ir::Type::SInt(width_input) => Ok(ir::Type::SInt(width_input)),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::And | ir::Prim::Or | ir::Prim::Xor => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 2",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::UInt(width_first), ir::Type::UInt(width_second)) => {
          Ok(ir::Type::UInt(max(width_first, width_second)))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(width_second)) => {
          Ok(ir::Type::UInt(max(width_first, width_second)))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Not => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match input_types[0].clone() {
        ir::Type::UInt(width) => Ok(ir::Type::UInt(width)),
        ir::Type::SInt(width) => Ok(ir::Type::UInt(width)),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Shl => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      if attrs.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, attr {} of length {} != 1",
          prim.ir_dump(),
          attrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", "),
          attrs.len()
        ),
          op_span,
        ))?;
      }
      let shift_width = attrs[0] as usize;
      match input_types[0].clone() {
        ir::Type::UInt(width) => {
          let new_width = width + shift_width as u32;
          Ok(ir::Type::UInt(new_width))
        }
        ir::Type::SInt(width) => {
          let new_width = width + shift_width as u32;
          Ok(ir::Type::SInt(new_width))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {} and attr {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            attrs[0]
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Shr => {
      if input_types.len() != 1 || attrs.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 1, attr {} of length {} != 1  ",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len(),
          attrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", "),
          attrs.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), attrs[0]) {
        (ir::Type::UInt(width_input), shift_width) => {
          Ok(ir::Type::UInt(
            max(width_input as i32 - shift_width as i32, 0) as u32,
          ))
        }
        (ir::Type::SInt(width_input), shift_width) => {
          Ok(ir::Type::SInt(
            max(width_input as i32 - shift_width as i32, 1) as u32,
          ))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and attr {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            attrs[0]
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::DShl => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 2",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::UInt(width_input), ir::Type::UInt(width_shift)) => Ok(
          ir::Type::UInt(width_input + pow(2, width_shift as usize) - 1),
        ),
        (ir::Type::SInt(width_input), ir::Type::UInt(width_shift)) => Ok(
          ir::Type::SInt(width_input + pow(2, width_shift as usize) - 1),
        ),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::DShr => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 2",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::UInt(width_input), ir::Type::UInt(_)) => {
          Ok(ir::Type::UInt(width_input))
        }
        (ir::Type::SInt(width_input), ir::Type::UInt(_)) => {
          Ok(ir::Type::SInt(width_input))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Cvt => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1",
            prim.ir_dump(),
            input_types
              .iter()
              .map(|t| t.ir_dump())
              .collect::<Vec<_>>()
              .join(", "),
            input_types.len()
          ),
          op_span,
        ))?;
      }
      match input_types[0].clone() {
        ir::Type::UInt(width) => Ok(ir::Type::SInt(width + 1)),
        ir::Type::SInt(width) => Ok(ir::Type::SInt(width)),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Neg => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 1",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match input_types[0].clone() {
        ir::Type::UInt(width) => Ok(ir::Type::SInt(width + 1)),
        ir::Type::SInt(width) => Ok(ir::Type::SInt(width + 1)),
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Andr | ir::Prim::Orr | ir::Prim::Xorr => {
      if input_types.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 1",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      Ok(ir::Type::UInt(1))
    }
    ir::Prim::Cat => {
      if input_types.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for prim {}, type {} of length {} != 2",
          prim.ir_dump(),
          input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len()
        ),
          op_span,
        ))?;
      }
      match (input_types[0].clone(), input_types[1].clone()) {
        (ir::Type::UInt(width_first), ir::Type::UInt(width_second)) => {
          Ok(ir::Type::UInt(width_first + width_second))
        }
        (ir::Type::SInt(width_first), ir::Type::SInt(width_second)) => {
          Ok(ir::Type::UInt(width_first + width_second))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and types {} and {}",
            prim.ir_dump(),
            input_types[0].ir_dump(),
            input_types[1].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Bits => {
      if input_types.len() != 1 || attrs.len() != 2 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1, attr {} of length {} != 2",
            prim.ir_dump(),
            input_types
              .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len(),
          attrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", "),
          attrs.len()
        ),
          op_span,
        ))?;
      }
      let high = attrs[0];
      let low = attrs[1];
      if high < low {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, attr {} < {}",
            prim.ir_dump(),
            attrs[0],
            attrs[1]
          ),
          op_span,
        ))?;
      }
      Ok(ir::Type::UInt(high - low + 1))
    }
    ir::Prim::Head => {
      if input_types.len() != 1 || attrs.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1, attr {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len(),
          attrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", "),
          attrs.len()
        ),
          op_span,
        ))?;
      }
      let width = attrs[0];
      match input_types[0].clone() {
        ir::Type::UInt(width_input) if width_input >= width => {
          Ok(ir::Type::UInt(width))
        }
        ir::Type::SInt(width_input) if width_input >= width => {
          Ok(ir::Type::UInt(width))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
    ir::Prim::Tail => {
      if input_types.len() != 1 || attrs.len() != 1 {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {}, type {} of length {} != 1, attr {} of length {} != 1",
            prim.ir_dump(),
            input_types
            .iter()
            .map(|t| t.ir_dump())
            .collect::<Vec<_>>()
            .join(", "),
          input_types.len(),
          attrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", "),
          attrs.len()
        ),
          op_span,
        ))?;
      }
      let width = attrs[0];
      match input_types[0].clone() {
        ir::Type::UInt(width_input) if width_input >= width => {
          Ok(ir::Type::UInt(width_input - width))
        }
        ir::Type::SInt(width_input) if width_input >= width => {
          Ok(ir::Type::UInt(width_input - width))
        }
        _ => Err(data.report_error_at_span(
          format!(
            "type infer failed for prim {} and type {}",
            prim.ir_dump(),
            input_types[0].ir_dump()
          ),
          op_span,
        ))?,
      }
    }
  }
}

pub fn op_infer_type(
  op: &ir::Op,
  input_types: Vec<ir::Type>,
  data: &VisitorData,
) -> Result<Vec<ir::Type>, anyhow::Error> {
  match op.inner() {
    ir::OpEnum::Nop(_) => Ok(vec![]),
    ir::OpEnum::Assign(_) => Ok(input_types),
    ir::OpEnum::Lit(ir::LitOp {
      value: ir::TypedLit { ty, .. },
      ..
    }) => Ok(vec![ty.clone()]),
    ir::OpEnum::Cmp(_) => Ok(vec![ir::Type::UInt(1)]),
    ir::OpEnum::Prim(ir::PrimOp { prim, attrs, .. }) => {
      Ok(vec![prim_type_infer(
        prim,
        input_types,
        attrs,
        op.span(),
        data,
      )?])
    }
    ir::OpEnum::Invoke(ir::InvokeOp { inst_rule, .. }) => {
      let module_name = data.resolve_path(&inst_rule.path);
      let rule_name = inst_rule.rule_name.clone();
      // let otype = data.circuit.rule_output_types(&module_name,
      // &rule_name)?;
      let otype = data.rule_output_types(&module_name, &rule_name)?;
      Ok(otype)
    }
    ir::OpEnum::Timed(_, op) => op_infer_type(op, input_types, data),
    ir::OpEnum::Block(body) => {
      if let Some(last_op) = body.ops.last() {
        let output_types = last_op
          .outputs()
          .map(|id| data.type_of(id).unwrap())
          .collect();
        Ok(output_types)
      } else {
        Ok(vec![])
      }
    }
    ir::OpEnum::If(if_op) => {
      match data.type_of(if_op.cond) {
        Some(ir::Type::UInt(1)) => {
          // _ => {
          let then_output_types =
            op_infer_type(&if_op.then_body, input_types.clone(), data)?;
          let combined_output_types = if let Some(else_body) = &if_op.else_body
          {
            // since abort is not supported, else body must has the same
            // output types as then body
            let else_output_types =
              op_infer_type(else_body, input_types.clone(), data)?;
            if then_output_types.len() != else_output_types.len() {
              return Err(data.report_error_at_span(
                format!(
                "type infer failed for IfOp, then body and else body have different number of outputs: {} vs {}",
                then_output_types.len(), else_output_types.len()
              ),
                op.span(),
              ))?;
            }
            let mut combined_output_types = vec![];
            for (i, (t1, t2)) in then_output_types
              .iter()
              .zip(else_output_types.iter())
              .enumerate()
            {
              if t1 != t2 {
                match (t1.clone(), t2.clone()) {
                  (ir::Type::UInt(w1), ir::Type::UInt(w2)) => {
                    combined_output_types.push(ir::Type::UInt(w1.max(w2)));
                  }
                  (ir::Type::SInt(w1), ir::Type::SInt(w2)) => {
                    combined_output_types.push(ir::Type::SInt(w1.max(w2)));
                  }
                  _ => {
                    return Err(data.report_error_at_span(
                      format!(
                      "type infer failed for IfOp, {}-th output of then-body and else-body have different/unmergeable types: {} vs {}",
                      i, t1.ir_dump(), t2.ir_dump()
                    ),
                    op.span(),
                  ))?;
                  }
                }
              } else {
                combined_output_types.push(t1.clone());
              }
            }
            combined_output_types
          } else {
            then_output_types
          };
          Ok(combined_output_types)
        }
        Some(x) => Err(data.report_error_at_span(
          format!(
            "IfOp's cond must be a bool [UInt(1)], but got {}.",
            x.ir_dump()
          ),
          op.span(),
        ))?,
        None => Err(data.report_error_at_span(
          format!("IfOp's cond must be a bool [UInt(1)], but got None.",),
          op.span(),
        ))?,
      }
    }
    ir::OpEnum::Field(ir::FieldOp { value, field, .. }) => match field {
      ir::Field::Name(name) => {
        let type_of_value =
          data.type_of(*value).ok_or(data.report_error_at_span(
            format!(
              "type infer failed for field {}, value {}'s type not found",
              name,
              data.print_value(*value)
            ),
            op.span(),
          ))?;
        if let ir::Type::Bundle(fields) = type_of_value.clone() {
          if let Some((_, field_type, _)) =
            fields.iter().find(|(f, _, _)| f == name)
          {
            Ok(vec![field_type.clone()])
          } else {
            Err(data.report_error_at_span(
              format!(
                "type infer failed for field {}, value {}'s type {} does not have field {}",
                name, data.print_value(*value), type_of_value.ir_dump(), name
              ),
              op.span(),
            ))?
          }
        } else {
          Err(data.report_error_at_span(
            format!(
              "type infer failed for field {}, value {}'s type {} is not a struct",
              name, data.print_value(*value), type_of_value.ir_dump()
            ),
            op.span(),
          ))?
        }
      }
      ir::Field::Index(index) => {
        let type_of_value =
          data.type_of(*value).ok_or(data.report_error_at_span(
            format!(
              "type infer failed for field {}, value {}'s type not found",
              index,
              data.print_value(*value)
            ),
            op.span(),
          ))?;
        if let ir::Type::Vector(element_type, size) = type_of_value {
          if *index < size {
            Ok(vec![*element_type])
          } else {
            Err(data.report_error_at_span(
              format!(
                "type infer failed for vector, index {} out of bounds ({})",
                index, size,
              ),
              op.span(),
            ))?
          }
        } else {
          Err(data.report_error_at_span(
            format!(
              "type infer failed for field {}, value {}'s type {} is not a vector",
              index, data.print_value(*value), type_of_value.ir_dump()
            ),
            op.span(),
          ))?
        }
      }
    },
    ir::OpEnum::Aggregate(ir::AggregateOp { values, fields, .. }) => {
      if values.len() != fields.len() {
        return Err(data.report_error_at_span(
          format!(
          "type infer failed for aggregate, values {} and fields {}",
          values
            .iter()
            .map(|v| data.print_value(*v))
            .collect::<Vec<_>>()
            .join(", "),
          fields
            .iter()
            .map(|f| f.ir_dump())
            .collect::<Vec<_>>()
            .join(", ")
        ),
          op.span(),
        ))?;
      }
      if values.len() == 0 {
        return Ok(vec![]);
      }
      let all_named = fields.iter().all(|f| matches!(f, ir::Field::Name(_)));
      let all_index = fields.iter().all(|f| matches!(f, ir::Field::Index(_)));
      let values_all_same_type = {
        let first_value_type = data.type_of(values[0]).unwrap();
        values
          .iter()
          .all(|v| data.type_of(*v).unwrap() == first_value_type)
      };
      if !all_named && !all_index {
        return Err(data.report_error_at_span(
          format!(
            "type infer failed for aggregate, the fields must be either all-named or all-index, but got {}",
            fields
              .iter()
              .map(|f| f.ir_dump())
              .collect::<Vec<_>>()
              .join(", ")
          ),
          op.span(),
        ))?;
      }

      if all_named {
        // infer the struct type
        let struct_type = ir::Type::bundle(
          fields
            .iter()
            .map(|f| {
              if let ir::Field::Name(name) = f {
                name
              } else {
                panic!("all_named is true, but field is not named");
              }
            })
            .zip(values.iter())
            .map(|(f, v)| {
              let value_type = data.type_of(*v).unwrap();
              (f.to_string(), value_type, false)
            })
            .collect::<Vec<_>>(),
        );
        Ok(vec![struct_type])
      } else {
        if !values_all_same_type {
          return Err(data.report_error_at_span(
            format!(
              "type infer failed for aggregate, the values must be of the same type, but got {}",
              values
                .iter()
                .map(|v| data.print_value(*v))
                .collect::<Vec<_>>()
                .join(", ")
            ),
            op.span(),
          ))?;
        }
        // infer the vector type
        let vector_type = ir::Type::vector(
          data.type_of(values[0]).unwrap(),
          fields.len() as u32,
        );
        Ok(vec![vector_type])
      }
    }
    ir::OpEnum::Return(ir::ReturnOp { values }) => {
      let output_types =
        values.iter().map(|id| data.type_of(*id).unwrap()).collect();
      Ok(output_types)
    }
    ir::OpEnum::Delay(_) => Ok(vec![ir::Type::UInt(1)]),
    ir::OpEnum::DynDelay(_) => Ok(vec![ir::Type::UInt(1)]),
    ir::OpEnum::Call(_) => Ok(vec![]),
    ir::OpEnum::Step(_) => Ok(vec![]),
    ir::OpEnum::Seq(_) => Ok(vec![]),
    ir::OpEnum::Par(_) => Ok(vec![]),
    ir::OpEnum::Branch(_) => Ok(vec![]),
    ir::OpEnum::For(_) => Ok(vec![]),
    ir::OpEnum::SimPrint(_) => Ok(vec![]),
    ir::OpEnum::SimExit(_) => Ok(vec![]),
    ir::OpEnum::SimFromInt(SimFromInt { res, a: _ }) => {
      Ok(vec![data.type_of(*res).unwrap()])
    }
  }
}

pub struct TypeInferPass {
  pub num_new_types: u32,
}

impl TypeInferPass {
  pub fn new() -> Self {
    Self { num_new_types: 0 }
  }

  fn infer_type(
    &mut self,
    op: &ir::Op,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    // log::debug!("infer type of {}", op.ir_dump_with(&data.module.values));
    let mut input_types = vec![];
    for input in op.inputs() {
      if let Some(itype) = data.type_of(input) {
        input_types.push(itype);
      }
    }
    let otype = op_infer_type(op, input_types, data)?;
    let outputs: Vec<_> = op.outputs().collect();
    if otype.len() != outputs.len() {
      return Err(data.report_error_at_op(
        format!(
          "op {} has {} outputs, but inferred {} types",
          op.ir_dump_with(&data.module.values),
          outputs.len(),
          otype.len()
        ),
        &op,
      ))?;
    }
    for (output, otype) in outputs.iter().zip(otype.iter()) {
      if data.type_of(*output).is_none() {
        self.num_new_types += 1;
        data.set_type(*output, otype.clone());
      }
    }

    Ok(())
    // Err("not implemented".to_string())
  }
}

impl Visitor for TypeInferPass {
  fn name() -> &'static str {
    "TypeInferPass"
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    let guard_ops =
      data.rule().guard().map(|op| op.clone()).collect::<Vec<_>>();
    for op in guard_ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        self.infer_type(&op, data)?;
      }
    }

    let ops = data.rule().ops().map(|op| op.clone()).collect::<Vec<_>>();
    for op in ops {
      let flatten = Self::flatten_op(&op);
      for op in flatten {
        self.infer_type(&op, data)?;
      }
    }

    // keep the rule as is
    Ok((vec![data.take_rule()], vec![]))
  }
}
