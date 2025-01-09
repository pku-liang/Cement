use crate::AssignOp;
use num::integer;
use std::collections::HashSet;

use super::*;
use std::hash::{Hash, Hasher};

/// Function inline pass
/// must be used before any other passes!
pub struct FunctionInlinePass {
  counter: usize,
}

impl FunctionInlinePass {
  pub fn new() -> Self {
    Self { counter: 0 }
  }

  pub fn process_function_inline(
    &mut self,
    op: &ir::Op,
    data: &mut VisitorData,
  ) -> Option<Vec<ir::Op>> {
    if let ir::OpEnum::Call(ir::CallOp {
      res,
      function_name,
      args,
      instances,
    }) = op.inner()
    {
      let function = data
        .circuit
        .functions()
        .find(|func| func.name == *function_name)
        .unwrap()
        .clone();

      let mut func_ops = function.ops.clone();
      let prefix = format!("inline{}", self.counter);
      self.counter += 1;

      let mut value_map: HashMap<ir::ValueId, ir::ValueId> = HashMap::new();

      for (old_value, new_value) in function.inputs.iter().zip(args.iter()) {
        value_map.insert(*old_value, *new_value);
      }

      for (old_instance, new_instance) in
        function.instances.iter().zip(instances.iter())
      {
        let old_name = old_instance.name.clone();
        func_ops.iter_mut().for_each(|func_op: &mut crate::Op| {
          func_op.replace_instance(old_name.clone(), new_instance.to_string());
        });
      }

      for op in func_ops.iter_mut() {
        let flatten = Self::flatten_op(&op);
        for op in flatten.iter() {
          for value_id in op.outputs() {
            let typ = value_id.ty(&function.values);
            let new_name = value_id
              .name(&function.values)
              .as_ref()
              .map(|name| format!("{}_{}", prefix, name));
            let new_value = data.module.values.insert(ir::Value {
              ty: typ,
              name: new_name,
            });
            value_map.insert(value_id, new_value);
          }
        }
      }

      for op in func_ops.iter_mut() {
        op.replace_value_with_map(&value_map);
      }

      if let Some(ir::OpEnum::Return(ir::ReturnOp { values })) =
        func_ops.clone().last().map(|op| op.inner())
      {
        func_ops.pop();
        for (old_value, new_value) in values.iter().zip(res.iter()) {
          data.module.values[*old_value].name = None;
          let op = ir::OpEnum::Assign(ir::AssignOp {
            res: new_value.clone(),
            value: old_value.clone(),
          });
          func_ops.push(op.into());
        }
      }

      Some(func_ops)
    } else {
      None
    }
  }

  pub fn process_function_inline_for_func(
    &mut self,
    op: &ir::Op,
    func: &mut ir::Function,
    circuit: &mut ir::Circuit,
  ) -> Option<Vec<ir::Op>> {
    if let ir::OpEnum::Call(ir::CallOp {
      res,
      function_name,
      args,
      instances,
    }) = op.inner()
    {
      let function = circuit
        .functions()
        .find(|func| func.name == *function_name)
        .unwrap()
        .clone();

      let mut func_ops: Vec<crate::Op> = function.ops.clone();
      let prefix: String = format!("inline{}", self.counter);
      self.counter += 1;

      let mut value_map: HashMap<ir::ValueId, ir::ValueId> = HashMap::new();

      for (old_value, new_value) in function.inputs.iter().zip(args.iter()) {
        value_map.insert(*old_value, *new_value);
      }

      for (old_instance, new_instance) in
        function.instances.iter().zip(instances.iter())
      {
        let old_name = old_instance.name.clone();
        func_ops.iter_mut().for_each(|func_op: &mut crate::Op| {
          func_op.replace_instance(old_name.clone(), new_instance.to_string());
        });
      }

      for op in func_ops.iter_mut() {
        let flatten = Self::flatten_op(&op);
        for op in flatten.iter() {
          for value_id in op.outputs() {
            let typ = value_id.ty(&function.values);
            let new_name = value_id
              .name(&function.values)
              .as_ref()
              .map(|name| format!("{}_{}", prefix, name));
            let new_value = func.values.insert(ir::Value {
              ty: typ,
              name: new_name,
            });
            value_map.insert(value_id, new_value);
          }
        }
      }

      for op in func_ops.iter_mut() {
        op.replace_value_with_map(&value_map);
      }

      if let Some(ir::OpEnum::Return(ir::ReturnOp { values })) =
        func_ops.clone().last().map(|op| op.inner())
      {
        func_ops.pop();
        for (old_value, new_value) in values.iter().zip(res.iter()) {
          func.values[*old_value].name = None;
          let op: crate::Op = ir::OpEnum::Assign(ir::AssignOp {
            res: new_value.clone(),
            value: old_value.clone(),
          })
          .into();
          func_ops.push(op);
        }
      }

      Some(func_ops)
    } else {
      None
    }
  }
}

impl Visitor for FunctionInlinePass {
  fn name() -> &'static str {
    "FunctionInlinePass"
  }

  fn after_pass(&mut self, circuit: &mut crate::Circuit) -> anyhow::Result<()> {
    // delete functions
    circuit.functions.clear();
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    let mut rule = data.take_rule().clone();
    let mut guard_replacements = vec![];

    for op in rule.guard_mut() {
      let flatten = Self::flatten_op(&op);
      for (index, op) in flatten.iter().enumerate() {
        if let Some(inlined_ops) = self.process_function_inline(&op, data) {
          guard_replacements.push((index, inlined_ops));
        }
      }
    }

    for (index, inlined_ops) in guard_replacements {
      rule.replace_guard_op(index, inlined_ops);
    }

    let mut op_replacements = vec![];
    for op in rule.ops_mut() {
      let flatten = Self::flatten_op(&op);
      for (index, op) in flatten.iter().enumerate() {
        if let Some(inlined_ops) = self.process_function_inline(&op, data) {
          op_replacements.push((index, inlined_ops));
        }
      }
    }

    for (index, inlined_ops) in op_replacements {
      rule.replace_op(index, inlined_ops);
    }

    data.rule = Some(rule);
    // Keep the rule as is
    Ok((vec![data.take_rule()], vec![]))
  }

  fn visit_function(
    &mut self,
    function: &mut ir::Function,
    circuit: &mut ir::Circuit,
  ) -> anyhow::Result<()> {
    let mut func_old = function.clone();

    for op in func_old.ops_mut() {
      let flatten = Self::flatten_op(&op);
      for op in flatten.iter() {
        let mut func_mirror = function.clone();
        if let Some(inlined_ops) =
          self.process_function_inline_for_func(&op, &mut func_mirror, circuit)
        {
          // op_replacements.push((index,  inlined_ops));
          func_mirror.replace_op(op, inlined_ops);
        }
        *function = func_mirror;
      }
    }

    Ok(())
  }
}
