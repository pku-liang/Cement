/// passes in this file should be used and tested after `mpart` pass
use std::cmp::{Ordering, Reverse};

use indexmap::IndexSet;
use log::info;

use crate::Cmp;

use super::*;

use super::*;

/// Module inline pass
/// 1. delete rules that have been partitioned
/// 2. determine whether a module is synthesized into a module or not
/// 3. inline all instances of modules that are not to be synthesized
pub struct InlinePass {
  pub rule_derive_table: IndexMap<RuleId, Vec<RuleId>>,
  pub modules_to_synthesize: IndexSet<String>,
}

impl InlinePass {
  pub fn new() -> Self {
    Self {
      rule_derive_table: IndexMap::new(),
      modules_to_synthesize: IndexSet::new(),
    }
  }
}

fn replace_rule_names(
  rule_rel: &mut ir::RuleRel,
  rule_name_map: &HashMap<String, String>,
) {
  match rule_rel {
    ir::RuleRel::Method { lhs, rhs, .. } => {
      for inst_rule in lhs {
        inst_rule.rule_name = rule_name_map
          .get(&inst_rule.rule_name)
          .cloned()
          .unwrap_or_else(|| inst_rule.rule_name.clone());
      }
      for inst_rule in rhs {
        inst_rule.rule_name = rule_name_map
          .get(&inst_rule.rule_name)
          .cloned()
          .unwrap_or_else(|| inst_rule.rule_name.clone());
      }
    }
    ir::RuleRel::Schedule(inst_rules) => {
      for inst_rule in inst_rules {
        inst_rule.rule_name = rule_name_map
          .get(&inst_rule.rule_name)
          .cloned()
          .unwrap_or_else(|| inst_rule.rule_name.clone());
      }
    }
  }
}

impl Visitor for InlinePass {
  fn name() -> &'static str {
    "mod-inline"
  }

  /// tb module should have no inline pass
  fn skip_tb() -> bool {
    true
  }

  // 1. delete rules that have been partitioned
  fn visit_rule_impl(
    &mut self,
    data: &mut VisitorData,
  ) -> Result<(Vec<crate::Rule>, Vec<crate::RuleRel>), anyhow::Error> {
    if self.rule_derive_table.contains_key(&data.rule_id()) {
      Ok((vec![], vec![]))
    } else {
      Ok((vec![data.take_rule()], vec![]))
    }
  }

  fn after_visit_rules(
    &mut self,
    data: &mut VisitorData,
  ) -> anyhow::Result<()> {
    // 2. determine whether a module is synthesized into a module or not
    let mut to_module = data.module.clone();
    let mut delete_instance = vec![];
    let mut inlined_rules = vec![];

    for instance in data.module.instances() {
      let from_module = data
        .circuit
        .module(&instance.module)
        .ok_or(data.report_error(format!("module {} not found", instance.module)))?;

      if !to_module.is_tb() && !from_module.to_be_synthesize() {
        log::info!(
          "\tinline instance {} of module {} to module {}",
          instance.name,
          instance.module,
          to_module.name
        );
        delete_instance.push(instance.clone());

        let mut cur_inlined_rules = vec![];
        let mut value_map: HashMap<ir::ValueId, ir::ValueId> = HashMap::new();
        let mut port_name_map: HashMap<String, String> = HashMap::new();

        // add input of the inlined module to new module
        for input in from_module.inputs() {
          let mut input_value = from_module.values.get(input).unwrap().clone();
          let new_name = format!(
            "_il_{}_{}",
            instance.name,
            data.print_value_in_module(input, &from_module)
          );
          let input_new_value =
            to_module.add_value(Some(new_name.clone()), input_value.ty);
          to_module.add_wire(input_new_value);
          value_map.insert(input.clone(), input_new_value);
          port_name_map
            .insert(data.print_value_in_module(input, &from_module), new_name);
        }

        // add output of the inlined module to new module
        for output in from_module.outputs() {
          let mut output_value =
            from_module.values.get(output).unwrap().clone();
          let new_name = format!(
            "_il_{}_{}",
            instance.name,
            data.print_value_in_module(output, &from_module)
          );
          let output_new_value =
            to_module.add_value(Some(new_name.clone()), output_value.ty);
          to_module.add_wire(output_new_value);
          value_map.insert(output.clone(), output_new_value);
          port_name_map
            .insert(data.print_value_in_module(output, &from_module), new_name);
        }

        // add wires of the inlined module to new module
        for wire in from_module.wires() {
          let mut wire_value = from_module.values.get(wire).unwrap().clone();
          let new_name = format!(
            "_il_{}_{}",
            instance.name,
            data.print_value_in_module(wire, &from_module)
          );
          let wire_new_value =
            to_module.add_value(Some(new_name.clone()), wire_value.ty);
          to_module.add_wire(wire_new_value);
          value_map.insert(wire.clone(), wire_new_value);
        }

        //add rules of the inlined module to new module and change the invoke
        // op
        let mut rule_name_map: HashMap<String, String> = HashMap::new();
        let rules = from_module
          .rules()
          .map(|rule| rule.clone())
          .collect::<Vec<_>>();
        for rule in rules {
          let mut new_rule = rule.clone();
          new_rule.name =
            format!("_il_{}_{}", instance.name, rule.name.clone());
          rule_name_map.insert(rule.name.clone(), new_rule.name.clone());

          // replace rule inputs/outputs with new values
          for input in new_rule.inputs_mut() {
            *input = value_map
              .get(input)
              .ok_or(data.report_error(format!("input {} not found in value_map", input)))?
              .clone();
          }

          for output in new_rule.outputs_mut() {
            if !value_map.contains_key(output) {
              println!(
                "{:#?}",
                value_map
                  .iter()
                  .map(|(k, v)| (format!(
                    "{} -> {}",
                    data.print_value_in_module(*k, &from_module),
                    data.print_value_in_module(*v, &to_module)
                  )))
                  .collect::<Vec<_>>()
                  .join("; ")
              );
              println!(
                "value {} not found in value_map",
                data.print_value_in_module(*output, &from_module)
              );
              println!("\n----\n{}", from_module.ir_dump());
            }
            *output = value_map
              .get(output)
              .ok_or(data.report_error(format!(
                "output {} not found in value_map",
                output
              )))?
              .clone();
          }

          for op in new_rule.guard_ops.iter_mut() {
            op_replace_value_with_map_new(
              op,
              &mut value_map,
              &mut to_module,
              from_module,
              &instance.name,
              data,
            );
          }

          for op in new_rule.ops.iter_mut() {
            op_replace_value_with_map_new(
              op,
              &mut value_map,
              &mut to_module,
              from_module,
              &instance.name,
              data,
            );
          }
          new_rule.is_private = true;
          cur_inlined_rules.push(new_rule);
        }

        // change the invoke op of the new module
        for rule in to_module.rules_mut() {
          for op in rule.guard_ops.iter_mut() {
            change_invoke_op_rule(
              op,
              instance.name.clone(),
              &rule_name_map,
              &port_name_map,
            );
          }
          for op in rule.ops.iter_mut() {
            change_invoke_op_rule(
              op,
              instance.name.clone(),
              &rule_name_map,
              &port_name_map,
            );
          }
        }

        // change the invoke op of the inlined rules
        for rule in cur_inlined_rules.iter_mut() {
          for op in rule.guard_ops.iter_mut() {
            change_invoke_op_rule(
              op,
              "self".to_string(),
              &rule_name_map,
              &port_name_map,
            );
          }
          for op in rule.ops.iter_mut() {
            change_invoke_op_rule(
              op,
              "self".to_string(),
              &rule_name_map,
              &port_name_map,
            );
          }
        }

        //change the rule rel of the new module
        for rule_rel in from_module.rule_rels() {
          let mut new_rule_rel = rule_rel.clone();
          replace_rule_names(&mut new_rule_rel, &rule_name_map);
          to_module.rule_rels.push(new_rule_rel);
        }

        let mut instance_name_map: HashMap<String, String> = HashMap::new();
        //add instances of the inlined module to new module
        for sub_instance in from_module.instances() {
          let mut new_instance = sub_instance.clone();
          new_instance.name =
            format!("_il_{}_{}", instance.name, new_instance.name);
          instance_name_map
            .insert(sub_instance.name.clone(), new_instance.name.clone());
          to_module.add_instance(new_instance.name, new_instance.module);
        }

        // change the invoke op to the inlined instances
        for rule in cur_inlined_rules.iter_mut() {
          for op in rule.guard_ops.iter_mut() {
            change_invoke_op_instance(op, &instance_name_map);
          }
          for op in rule.ops.iter_mut() {
            change_invoke_op_instance(op, &instance_name_map);
          }
        }
        inlined_rules.extend(cur_inlined_rules);
      }
    }

    //remove the instance
    match &mut to_module.body {
      ir::ModuleBody::Native(body) => {
        body
          .instances
          .retain(|inst| !delete_instance.contains(inst));
      }
      _ => {}
    }

    inlined_rules.extend(std::mem::take(&mut to_module.rules));
    std::mem::swap(&mut to_module.rules, &mut inlined_rules);
    data.module = to_module;

    if data.module.to_be_synthesize() {
      log::debug!("module {} is to be synthesized", data.module.name);
      self.modules_to_synthesize.insert(data.module.name.clone());
    }

    // 3. inline all instances of modules that are not to be synthesized
    // @nanbing: do this!
    // for the example of `static.cmt`, if `add2_2stage` is not to be
    // synthesized (look at the method `Module::to_be_synthesize`),
    // then we should inline all instances of `add2_2stage`
    // to be specific, we should inline `add2` in `add3_3stage`:
    // 1) we add `add2_r1` to `add3_3stage`, where the first `_i` means the
    //    instance comes from a inline pass, and `add2` is the name of the
    //    original instance, and `r1` is the name of the sub-instance in
    //    `add2_2stage`;
    // 2) we add two rules `_il_add2_do_s_0` and `_il_add2_do_s_1` (`do_s_0` and
    //    `do_s_1` are two single-cycle rules produced by `mpart`) to
    //    `add3_3stage`:
    //   2.1) add inputs/outputs as wires: `_il_add2_a` and `_il_add2_out`
    // should be added to `add3_3stage` as wires (by `Module::add_wire`)
    // 2.2) add rules: `_il_add2_do_s_0` and `_il_add2_do_s_1` should be
    // added to `add3_3stage` as rules, every rule should be marked as
    // private (`Rule::is_private = true`)
    Ok(())
  }

  fn after_visit_modules(
    &mut self,
    circuit: &mut crate::Circuit,
  ) -> anyhow::Result<()> {
    circuit.modules.retain(|m| {
      self.modules_to_synthesize.contains(&m.name)
        || m.is_tb()
        || m.is_virtual()
    });
    Ok(())
  }
}

pub fn change_invoke_op_rule(
  op: &mut ir::Op,
  instance_name: String,
  rule_name_map: &HashMap<String, String>,
  port_name_map: &HashMap<String, String>,
) {
  match op.inner_mut() {
    ir::OpEnum::Invoke(invoke_op) => {
      let invoke_op_clone = invoke_op.clone();
      if let Some(name) = invoke_op_clone.inst_rule.canonicalize().path.last() {
        if *name == instance_name {
          invoke_op.inst_rule.path.clear();
          if let Some(new_rule_name) =
            rule_name_map.get(&invoke_op_clone.inst_rule.rule_name)
          {
            invoke_op.inst_rule.rule_name = new_rule_name.clone();
          } else {
            // here it must be _get_res_xxx or _feed_arg_xxx invokes
            let invoke_rule_name = invoke_op.inst_rule.rule_name.clone();
            if !invoke_rule_name.starts_with("_get_res_")
              && !invoke_rule_name.starts_with("_feed_arg_")
            {
              log::error!("when replace invoke op to {}", instance_name);
              log::error!(
                "only _get_res_xxx or _feed_arg_xxx invokes are allowed to be used without declaration: {}",
                invoke_rule_name
              );
              log::error!("{:#?}", rule_name_map);
              panic!("only _get_res_xxx or _feed_arg_xxx invokes are allowed to be used without declaration: {}", invoke_rule_name);
            }

            // split the invoke_rule_name into prefix and port_name, such as
            // _feed_arg_xxx -> ("_feed_arg_", "xxx")
            if invoke_rule_name.starts_with("_feed_arg_") {
              let old_port = invoke_rule_name[10..].to_string();
              let new_port =
                port_name_map.get(&old_port).unwrap_or(&old_port).clone();
              invoke_op.inst_rule.rule_name = format!("_feed_arg_{}", new_port);

              // change feed_arg into wire.write
              let wire_inst_name = format!("_wire_{}", new_port);
              invoke_op.inst_rule =
                ir::InstRule::new(vec![wire_inst_name, "write".to_string()]);
            } else {
              let old_port = invoke_rule_name[9..].to_string();
              let new_port =
                port_name_map.get(&old_port).unwrap_or(&old_port).clone();
              invoke_op.inst_rule.rule_name = format!("_get_res_{}", new_port);

              // change get_res into wire.read
              let wire_inst_name = format!("_wire_{}", new_port);
              invoke_op.inst_rule =
                ir::InstRule::new(vec![wire_inst_name, "read".to_string()]);
            }
          }
        }
      }
    }
    ir::OpEnum::If(if_op) => {
      change_invoke_op_rule(
        &mut if_op.then_body,
        instance_name.clone(),
        rule_name_map,
        port_name_map,
      );
      if let Some(else_body) = &mut if_op.else_body {
        change_invoke_op_rule(
          else_body,
          instance_name,
          rule_name_map,
          port_name_map,
        );
      }
    }
    ir::OpEnum::Block(body_op) => {
      for op in &mut body_op.ops {
        change_invoke_op_rule(
          op,
          instance_name.clone(),
          rule_name_map,
          port_name_map,
        );
      }
    }
    ir::OpEnum::Timed(_, _) => {
      unreachable!("Timed op should be removed in mpart pass");
    }
    _ => {}
  }
}

pub fn change_invoke_op_instance(
  op: &mut ir::Op,
  instance_name_map: &HashMap<String, String>,
) {
  match op.inner_mut() {
    ir::OpEnum::Invoke(invoke_op) => {
      let invoke_op_clone = invoke_op.clone();
      let instance_name =
        invoke_op_clone.inst_rule.canonicalize().path.join(".");

      log::debug!("try to change invoke op instance: {}", instance_name);

      if let Some(new_instance_name) = instance_name_map.get(&instance_name) {
        invoke_op.inst_rule.path = vec![new_instance_name.clone()];
      }
    }
    ir::OpEnum::If(if_op) => {
      change_invoke_op_instance(&mut if_op.then_body, instance_name_map);
      if let Some(else_body) = &mut if_op.else_body {
        change_invoke_op_instance(else_body, instance_name_map);
      }
    }
    ir::OpEnum::Block(body_op) => {
      for op in &mut body_op.ops {
        change_invoke_op_instance(op, instance_name_map);
      }
    }
    ir::OpEnum::Timed(_, _) => {
      unreachable!("Timed op should be removed in mpart pass");
    }
    _ => {}
  }
}

pub fn op_replace_value_with_map_new(
  op: &mut ir::Op,
  replacements: &mut HashMap<ir::ValueId, ir::ValueId>,
  to_module: &mut ir::Module,
  from_module: &ir::Module,
  instance_name: &str,
  data: &VisitorData,
) {
  match op.inner_mut() {
    ir::OpEnum::Cmp(cmp_op) => {
      if let Some(&new_id) = replacements.get(&cmp_op.a) {
        cmp_op.a = new_id;
      }
      if let Some(&new_id) = replacements.get(&cmp_op.b) {
        cmp_op.b = new_id;
      }
      let res_value = from_module.values.get(cmp_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(cmp_op.res.clone(), res_new_value);
      cmp_op.res = res_new_value;
    }
    ir::OpEnum::Prim(prim_op) => {
      for input in &mut prim_op.inputs {
        if let Some(&new_id) = replacements.get(input) {
          *input = new_id;
        }
      }
      let res_value = from_module.values.get(prim_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(prim_op.res.clone(), res_new_value);
      prim_op.res = res_new_value;
    }
    ir::OpEnum::Lit(lit_op) => {
      let res_value = from_module.values.get(lit_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(lit_op.res.clone(), res_new_value);
      lit_op.res = res_new_value;
    }
    ir::OpEnum::Invoke(invoke_op) => {
      for arg in &mut invoke_op.args {
        if let Some(&new_id) = replacements.get(arg) {
          *arg = new_id;
        }
      }
      for res in &mut invoke_op.res {
        let res_value = from_module.values.get(*res).unwrap().clone();
        let new_name =
          res_value.name.clone().map(|name| format!("_il_{}", name));
        let res_new_value = to_module.add_value(new_name, res_value.ty);
        replacements.insert(res.clone(), res_new_value);
        *res = res_new_value;
      }
    }
    ir::OpEnum::Assign(assign_op) => {
      if let Some(&new_id) = replacements.get(&assign_op.value) {
        assign_op.value = new_id;
      }
      let res_value = from_module.values.get(assign_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(assign_op.res.clone(), res_new_value);
      assign_op.res = res_new_value;
    }
    ir::OpEnum::Delay(delay_op) => {
      let res_value = from_module.values.get(delay_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(delay_op.res.clone(), res_new_value);
      delay_op.res = res_new_value;
    }
    ir::OpEnum::DynDelay(delay_op) => {
      let res_value = from_module.values.get(delay_op.res).unwrap().clone();
      let new_name = res_value.name.clone().map(|name| format!("_il_{}", name));
      let res_new_value = to_module.add_value(new_name, res_value.ty);
      replacements.insert(delay_op.res.clone(), res_new_value);
      delay_op.res = res_new_value;
    }
    ir::OpEnum::If(if_op) => {
      if let Some(&new_id) = replacements.get(&if_op.cond) {
        if_op.cond = new_id;
      }
      for res in &mut if_op.res {
        let res_value = from_module.values.get(*res).unwrap().clone();
        let new_name =
          res_value.name.clone().map(|name| format!("_il_{}", name));
        let res_new_value = to_module.add_value(new_name, res_value.ty);
        replacements.insert(res.clone(), res_new_value);
        *res = res_new_value;
      }
      op_replace_value_with_map_new(
        &mut if_op.then_body,
        replacements,
        to_module,
        from_module,
        instance_name,
        data,
      );
      if let Some(else_body) = &mut if_op.else_body {
        op_replace_value_with_map_new(
          else_body,
          replacements,
          to_module,
          from_module,
          instance_name,
          data,
        );
      }
    }
    ir::OpEnum::Block(body_op) => {
      for op in &mut body_op.ops {
        op_replace_value_with_map_new(
          op,
          replacements,
          to_module,
          from_module,
          instance_name,
          data,
        );
      }
    }
    ir::OpEnum::Timed(_, op) => {
      op_replace_value_with_map_new(
        op,
        replacements,
        to_module,
        from_module,
        instance_name,
        data,
      );
    }
    ir::OpEnum::Return(return_op) => {
      for arg in return_op.values.iter_mut() {
        if let Some(&new_id) = replacements.get(arg) {
          *arg = new_id;
        }
      }
    }
    _ => {}
  }
}