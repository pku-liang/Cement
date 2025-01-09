use builtin::{fifo1_gen, shift_reg_gen, wire_gen};
use indexmap::IndexSet;

use crate::Op;

use super::*;

/// PortResolvePass add methods for args and results, and convert wires into
/// instances. This pass only handles single-cycle rules.
pub struct PortResolvePass {
  wire_table: IndexMap<ir::ValueId, InstRule>,
  invoke_wire_table: IndexMap<
    String,
    (
      (Vec<(usize, ir::ValueId)>, Vec<(usize, ir::ValueId)>),
      (Vec<(usize, ir::ValueId)>, Vec<(usize, ir::ValueId)>),
    ),
  >,
}

impl PortResolvePass {
  pub fn new() -> Self {
    Self {
      wire_table: IndexMap::new(),
      invoke_wire_table: IndexMap::new(),
    }
  }
}

impl Visitor for PortResolvePass {
  fn name() -> &'static str {
    "port-resolve"
  }

  /// tb module should not have port resolve pass
  fn skip_tb() -> bool {
    true
  }

  fn before_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    self.wire_table.clear();
    self.invoke_wire_table.clear();
    let _data = data;
    Ok(())
  }

  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<ir::Rule>, Vec<ir::RuleRel>), anyhow::Error> {
    if !data.rule().is_single_cycle() {
      return Err(data.report_error(format!(
        "PortResolvePass only handles single-cycle rules, but rule {} is not single-cycle",
        data.rule().name
      )))?;
    }
    if data.module.is_ext() {
      return Ok((vec![data.take_rule()], vec![]));
    }

    let mut old_ops = std::mem::take(&mut data.rule_mut().ops);

    let mut pre_ops = vec![];
    let mut post_ops = vec![];

    let module_wires = data.module.wires().collect::<IndexSet<_>>();
    // if rules take wires as input, we need to convert them into wire.read
    // invokes
    let mut value_replace_map = IndexMap::new();
    for arg in data.rule().inputs() {
      if module_wires.contains(&arg) {
        let wire_read_value = data.module.add_value(None, data.type_of(arg));
        pre_ops.push(ir::Op::invoke(
          ir::InstRule::new(vec![
            format!("_wire_{}", data.print_value(arg)),
            "read".to_string(),
          ]),
          vec![],
          vec![wire_read_value],
        ));
        value_replace_map.insert(arg, wire_read_value);
      }
    }

    let (wire_inputs, port_inputs): (Vec<_>, Vec<_>) = data
      .rule()
      .inputs()
      .into_iter()
      .enumerate()
      .partition(|(_, v)| module_wires.contains(v));

    let invoke_wire_input = (wire_inputs, port_inputs);

    // delete wires from rule inputs
    data
      .rule_mut()
      .inputs_mut()
      .retain(|v| !module_wires.contains(v));

    // replace usage of input wires in old ops
    old_ops.iter_mut().for_each(|op| {
      let num_inputs = op.num_inputs();
      for i in 0..num_inputs {
        let old_input = op.input(i);
        if let Some(wire_read_value) = value_replace_map.get(&old_input) {
          *op.input_mut(i) = wire_read_value.clone();
        }
      }
    });

    let (wire_outputs, port_outputs): (Vec<_>, Vec<_>) = data
      .rule()
      .outputs()
      .into_iter()
      .zip(
        old_ops
          .last()
          .map(|op| op.outputs().collect::<Vec<_>>())
          .unwrap_or_default(),
      )
      .enumerate()
      .partition(|(_, (v, _))| module_wires.contains(v));

    let invoke_wire_output: (
      Vec<(usize, ir::ValueId)>,
      Vec<(usize, ir::ValueId)>,
    ) = (
      wire_outputs
        .clone()
        .into_iter()
        .map(|(i, (v, _))| (i, v))
        .collect(),
      port_outputs
        .clone()
        .into_iter()
        .map(|(i, (_, ret_value))| (i, ret_value))
        .collect(),
    );

    if invoke_wire_input.0.len() > 0 || invoke_wire_output.0.len() > 0 {
      self.invoke_wire_table.insert(
        data.rule().name.clone(),
        (invoke_wire_input, invoke_wire_output),
      );
    }

    let mut last_op = old_ops.pop();

    if let Some(old_last_op) = last_op {
      if !old_last_op.is_return() {
        old_ops.push(old_last_op);
      }
      last_op = Some(ir::Op::ret(
        port_outputs
          .into_iter()
          .map(|(_, (_, ret_value))| ret_value)
          .collect(),
      ));
    }

    for (_, (wire_value, ret_value)) in wire_outputs {
      post_ops.push(ir::Op::invoke(
        ir::InstRule::new(vec![
          format!("_wire_{}", data.print_value(wire_value)),
          "write".to_string(),
        ]),
        vec![ret_value],
        vec![],
      ));
    }

    // delete wires from rule outputs
    data
      .rule_mut()
      .outputs_mut()
      .retain(|v| !module_wires.contains(v));

    // for rule input, invoke a read_arg method at beginning
    for arg in data.rule().inputs() {
      pre_ops.push(ir::Op::invoke(
        ir::InstRule::new(vec![format!("_read_arg_{}", data.print_value(arg))]),
        vec![],
        vec![],
      ));
    }

    // for rule output, invoke a write_res method at the end
    for res in data.rule().outputs() {
      post_ops.push(ir::Op::invoke(
        ir::InstRule::new(vec![format!(
          "_write_res_{}",
          data.print_value(res)
        )]),
        vec![],
        vec![],
      ));
    }

    pre_ops.extend(old_ops);
    pre_ops.extend(post_ops);
    pre_ops.extend(last_op);

    std::mem::swap(&mut data.rule_mut().ops, &mut pre_ops);

    Ok((vec![data.take_rule()], vec![]))
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(), anyhow::Error> {
    if data.module.is_ext() {
      return Ok(());
    }

    self.handle_ports(data)?;
    self.handle_wires(data)?;

    Ok(())
  }
}

impl PortResolvePass {
  // add read_arg, feed_arg, write_res, get_res methods to the module
  //   they are all private methods
  fn handle_ports(&mut self, data: &mut VisitorData) -> Result<(), anyhow::Error> {
    let mut extra_rules = vec![];
    let mut extra_rule_rels = vec![];
    for arg in data.module.inputs() {
      let read_arg_name = format!("_read_arg_{}", data.print_value(arg));
      let feed_arg_name = format!("_feed_arg_{}", data.print_value(arg));
      // read_arg is empty, its only a placeholder
      let read_arg = ir::Rule::method(
        read_arg_name.clone(),
        vec![],
        vec![],
        Some(false),
        vec![],
        vec![],
        ir::RuleTiming::SingleCycle,
      )
      .to_be_private();
      // feed_arg is for outside data to feed in by input arg
      let feed_arg = ir::Rule::method(
        feed_arg_name.clone(),
        vec![arg],
        vec![],
        Some(true),
        vec![],
        vec![],
        ir::RuleTiming::SingleCycle,
      )
      .to_be_private();
      extra_rules.push(read_arg);
      extra_rules.push(feed_arg);
      extra_rule_rels.push(ir::RuleRel::method(
        cmtir::MethodRel::SA,
        vec![InstRule::new(vec![feed_arg_name.clone()]).canonicalize()],
        vec![InstRule::new(vec![read_arg_name.clone()]).canonicalize()],
      ));
      extra_rule_rels.push(ir::RuleRel::method(
        cmtir::MethodRel::C,
        vec![InstRule::new(vec![feed_arg_name.clone()]).canonicalize()],
        vec![InstRule::new(vec![feed_arg_name.clone()]).canonicalize()],
      ));
    }

    for res in data.module.outputs() {
      let write_res_name = format!("_write_res_{}", data.print_value(res));
      let get_res_name = format!("_get_res_{}", data.print_value(res));
      let write_res = ir::Rule::method(
        write_res_name.clone(),
        vec![],
        vec![],
        Some(true),
        vec![],
        vec![],
        ir::RuleTiming::SingleCycle,
      )
      .to_be_private();
      let get_res = ir::Rule::method(
        get_res_name.clone(),
        vec![],
        vec![res],
        Some(false),
        vec![],
        vec![],
        ir::RuleTiming::SingleCycle,
      )
      .to_be_private();
      extra_rules.push(write_res);
      extra_rules.push(get_res);
      extra_rule_rels.push(ir::RuleRel::method(
        cmtir::MethodRel::SA,
        vec![InstRule::new(vec![write_res_name.clone()]).canonicalize()],
        vec![InstRule::new(vec![get_res_name.clone()]).canonicalize()],
      ));
      extra_rule_rels.push(ir::RuleRel::method(
        cmtir::MethodRel::C,
        vec![InstRule::new(vec![write_res_name.clone()]).canonicalize()],
        vec![InstRule::new(vec![write_res_name.clone()]).canonicalize()],
      ));
    }

    data.module.rules_mut().extend(extra_rules.into_iter());
    data
      .module
      .rule_rels_mut()
      .extend(extra_rule_rels.into_iter());

    Ok(())
  }

  // convert wires into instances
  fn handle_wires(&mut self, data: &mut VisitorData) -> Result<(), anyhow::Error> {
    let mut new_instances = vec![];
    for wire in data.module.wires() {
      let wire_module = wire_gen(
        data
          .type_of(wire)
          .ok_or(data.report_error(format!("wire {} has no type", wire)))?,
      );
      let wire_module_name = wire_module.name.clone();
      data.circuit.add_module(wire_module);
      let wire_inst_name = format!("_wire_{}", data.print_value(wire));
      new_instances
        .push(ir::InstDef::new(wire_inst_name.clone(), wire_module_name));

      self.wire_table.insert(
        wire,
        InstRule {
          path: vec![wire_inst_name],
          rule_name: "".to_string(),
        },
      );
    }

    data.module.wires.clear();

    // eprintln!(
    //   "[{}] new wires instances: {}",
    //   data.module.name,
    //   new_instances
    //     .iter()
    //     .map(|inst| inst.ir_dump())
    //     .collect::<Vec<_>>()
    //     .join(", ")
    // );

    data
      .module
      .instances_mut()
      .extend(new_instances.into_iter());

    // change invokes to rules with wires as inputs and outputs
    data.module.rules_mut().iter_mut().for_each(|rule| {
      for op in rule.ops.iter_mut().chain(rule.guard_ops.iter_mut()) {
        if let Some(InvokeOp {
          res,
          inst_rule,
          args,
        }) = op.clone().as_invoke_op()
        {
          if !inst_rule.is_self() {
            continue;
          };
          // log::debug!("look at invoke {}", inst_rule.to_string());
          if let Some(((wire_input, port_input), (wire_output, port_output))) =
            self.invoke_wire_table.get(&inst_rule.rule_name)
          {
            log::debug!(
              "\t\treplace invoke {}'s wire ports",
              inst_rule.to_string()
            );
            let mut new_ops = vec![];
            // add wire.write for wire_input
            for (i, wire_input) in wire_input.iter() {
              new_ops.push(ir::Op::invoke(
                self
                  .wire_table
                  .get(wire_input)
                  .unwrap()
                  .clone()
                  .with_rule_name("write".to_string()),
                vec![args[*i]],
                vec![],
              ));
            }
            // add new invoke with port_input and port_output
            new_ops.push(ir::Op::invoke(
              inst_rule,
              port_input.into_iter().map(|(i, _)| args[*i]).collect(),
              port_output.into_iter().map(|(i, _)| res[*i]).collect(),
            ));
            // add wire.read for wire_output
            for (i, wire_output) in wire_output.iter() {
              new_ops.push(ir::Op::invoke(
                self
                  .wire_table
                  .get(wire_output)
                  .unwrap()
                  .clone()
                  .with_rule_name("read".to_string()),
                vec![],
                vec![res[*i]],
              ));
            }
            *op = Op::block(new_ops);
          }
        }
      }
    });

    Ok(())
  }
}
