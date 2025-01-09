use super::*;

#[test]
fn test_neg_sint() -> anyhow::Result<()> {
  let mut module = Module::with_name("top".to_string()).as_top();

  // rule do: () -> (c: sint8)

  let c = module.add_value(Some("c".to_string()), Some(ir::Type::SInt(8)));
  let res = module.add_value(Some("res".to_string()), None);

  let do_ = ir::Rule::method(
    "do".to_string(),
    vec![],
    vec![c],
    None,
    vec![],
    vec![ir::Op::lit(TypedLit::from(-42_i8), res)],
    ir::RuleTiming::SingleCycle,
  );

  module.add_output(c);

  module.add_rule(do_);

  let circuit = Circuit::new("top".to_string(), vec![module]);

  println!("{}", circuit.ir_dump());

  let (_, fir) = to_fir_pipeline(circuit)?;
  run_firtool(fir, &path_in_crate_dir("./test_neg_sint"))?;
  Ok(())
}

#[test]
fn test_vector_failed() -> anyhow::Result<()> {
  let mut module = Module::with_name("top".to_string());

  // rule do: (a: i32x4, b: i32x4) -> (c: i32x4)
  let a = module.add_value(
    Some("a".to_string()),
    Some(ir::Type::vector(ir::Type::UInt(32), 4)),
  );
  let b = module.add_value(
    Some("b".to_string()),
    Some(ir::Type::vector(ir::Type::UInt(32), 4)),
  );
  let c = module.add_value(
    Some("c".to_string()),
    Some(ir::Type::vector(ir::Type::UInt(32), 4)),
  );
  let res = module.add_value(Some("res".to_string()), None);

  let add = ir::Op::add(a, b, res);

  let do_ = ir::Rule::method(
    "do".to_string(),
    vec![a, b],
    vec![c],
    None,
    vec![],
    vec![add],
    ir::RuleTiming::SingleCycle,
  );

  module.add_rule(do_);

  let circuit = Circuit::new("top".to_string(), vec![module]);

  println!("{}", circuit.ir_dump());

  match to_fir_pipeline(circuit) {
    Ok(_) => {
      return Err(anyhow::anyhow!("should fail by type inference"));
    }
    Err(e) => {
      println!("{}", e);
    }
  };

  // println!("{}", fir);
  Ok(())
}

#[test]
fn test_field() -> anyhow::Result<()> {
  let mut module = Module::with_name("top".to_string()).as_top();

  // rule do: (a: i32x4, b: {x: i32}) -> (c: i32)
  let a = module.add_value(
    Some("a".to_string()),
    Some(ir::Type::vector(ir::Type::UInt(32), 4)),
  );
  let b = module.add_value(
    Some("b".to_string()),
    Some(ir::Type::bundle(vec![(
      "x".to_string(),
      ir::Type::UInt(32),
      false,
    )])),
  );
  let c = module.add_value(Some("c".to_string()), Some(ir::Type::UInt(32)));
  let res = module.add_value(Some("res".to_string()), None);

  let a_0 = module.add_value(None, None);
  let b_x = module.add_value(None, None);
  let a_field = ir::Op::field(a, Field::Index(0), a_0);
  let b_field = ir::Op::field(b, Field::Name("x".to_string()), b_x);
  let add = ir::Op::add(a_0, b_x, res);

  let do_ = ir::Rule::method(
    "do".to_string(),
    vec![a, b],
    vec![c],
    None,
    vec![],
    vec![a_field, b_field, add],
    ir::RuleTiming::SingleCycle,
  );

  module.add_input(a);
  module.add_input(b);
  module.add_output(c);

  module.add_rule(do_);

  let circuit = Circuit::new("top".to_string(), vec![module]);

  println!("{}", circuit.ir_dump());

  let (_, fir) = to_fir_pipeline(circuit)?;
  run_firtool(fir, &path_in_crate_dir("./test_field"))?;
  Ok(())
}

#[test]
fn test_aggregate() -> anyhow::Result<()> {
  let mut module = Module::with_name("top".to_string()).as_top();

  // rule do: (a: i32x4, b: {x: i32}) -> (struct: {a: i32x4, b: {x: i32}, vec:
  // i32x4x4})
  let a = module.add_value(
    Some("a".to_string()),
    Some(ir::Type::vector(ir::Type::UInt(32), 4)),
  );
  let b = module.add_value(
    Some("b".to_string()),
    Some(ir::Type::bundle(vec![(
      "x".to_string(),
      ir::Type::UInt(32),
      false,
    )])),
  );
  let c = module.add_value(
    Some("c".to_string()),
    Some(ir::Type::bundle(vec![
      (
        "a".to_string(),
        ir::Type::vector(ir::Type::UInt(32), 4),
        false,
      ),
      (
        "b".to_string(),
        ir::Type::bundle(vec![("x".to_string(), ir::Type::UInt(32), false)]),
        false,
      ),
    ])),
  );
  let d = module.add_value(
    Some("d".to_string()),
    Some(ir::Type::vector(ir::Type::vector(ir::Type::UInt(32), 4), 4)),
  );

  let new_struct = module.add_value(None, None);
  let new_vector = module.add_value(None, None);

  let agg_struct = ir::Op::create_struct(
    vec![a, b],
    vec!["a".to_string(), "b".to_string()],
    new_struct,
  );
  let agg_vector = ir::Op::create_vector(vec![a, a, a, a], new_vector);
  let return_op = ir::Op::ret(vec![new_struct, new_vector]);

  let do_ = ir::Rule::method(
    "do".to_string(),
    vec![a, b],
    vec![c, d],
    None,
    vec![],
    vec![agg_struct, agg_vector, return_op],
    ir::RuleTiming::SingleCycle,
  );

  module.add_input(a);
  module.add_input(b);
  module.add_output(c);
  module.add_output(d);

  module.add_rule(do_);

  let circuit = Circuit::new("top".to_string(), vec![module]);

  println!("{}", circuit.ir_dump());

  // match to_fir_pipeline(circuit) {
  //   Ok(_) => {
  //     return Err("should fail since FIRRTL does not support
  // struct-/vector-create".to_string());   }
  //   Err(e) => {
  //     println!("{}", e);
  //   }
  // };
  let (_, fir) = to_fir_pipeline(circuit)?;
  run_firtool(fir, &path_in_crate_dir("./test_aggregate"))?;
  Ok(())
}

#[test]
fn test_flip() -> anyhow::Result<()> {
  let mut module = Module::with_name("top".to_string()).as_top();

  // rule do: (a: {x: i32, flip y: i32}) -> (c: {x: i32, flip y: i32})
  let a = module.add_value(
    Some("a".to_string()),
    Some(ir::Type::bundle(vec![
      ("x".to_string(), ir::Type::UInt(32), false),
      ("y".to_string(), ir::Type::UInt(32), true),
    ])),
  );
  let c = module.add_value(
    Some("c".to_string()),
    Some(ir::Type::bundle(vec![
      ("x".to_string(), ir::Type::UInt(32), false),
      ("y".to_string(), ir::Type::UInt(32), true),
    ])),
  );
  let return_op = ir::Op::ret(vec![a]);

  let do_ = ir::Rule::method(
    "do".to_string(),
    vec![a],
    vec![c],
    None,
    vec![],
    vec![return_op],
    ir::RuleTiming::SingleCycle,
  );

  module.add_input(a);
  module.add_output(c);

  module.add_rule(do_);

  let circuit = Circuit::new("top".to_string(), vec![module]);

  println!("{}", circuit.ir_dump());

  let (_, fir) = to_fir_pipeline(circuit)?;
  run_firtool(fir, &path_in_crate_dir("./test_flip"))?;
  Ok(())
}
