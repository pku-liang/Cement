use super::*;

pub trait ModuleExt {
  fn value_to_cpp(&self, id: ValueId) -> String;
  fn value_to_rust(&self, id: ValueId) -> String;
}

impl ModuleExt for ir::Module {
  fn value_to_cpp(&self, id: ValueId) -> String {
    let s = id
      .ir_dump_with(&self.values)
      .split_once(":")
      .unwrap()
      .0
      .to_string()
      .replace("%", "");
    if s.chars().next().unwrap().is_numeric() {
      format!("v{}", s)
    } else {
      s
    }
  }

  fn value_to_rust(&self, id: ValueId) -> String {
    id.ir_dump_with(&self.values)
      .split_once(":")
      .unwrap()
      .0
      .to_string()
      .replace("%", "t_")
  }
}

#[cfg(test)]
mod tests {

  use crate::{path_in_crate_dir, run_firtool, to_fir_pipeline};

  use super::*;
  use cmtir::utils::setup_logger_with_level;

  #[test]
  fn test_flatten_port() -> anyhow::Result<()> {
    setup_logger_with_level("debug".to_string());
    let mut module = Module::with_name("top".to_string()).as_top();

    // rule do: (a: vector<4, {x: i32, flip y: i32}>) -> (c: vector<2, {x: i32,
    // flip y: i32}>)
    let a = module.add_value(
      Some("a".to_string()),
      Some(ir::Type::vector(
        ir::Type::bundle(vec![
          ("x".to_string(), ir::Type::UInt(32), false),
          ("y".to_string(), ir::Type::UInt(32), true),
        ]),
        4,
      )),
    );
    let c = module.add_value(
      Some("c".to_string()),
      Some(ir::Type::vector(
        ir::Type::bundle(vec![
          ("x".to_string(), ir::Type::UInt(32), false),
          ("y".to_string(), ir::Type::UInt(32), true),
        ]),
        2,
      )),
    );

    module.add_input(a);
    module.add_output(c);


    let circuit = Circuit::new("top".to_string(), vec![module]);

    let (cmtir, fir) = to_fir_pipeline(circuit)?;
    println!("{}", cmtir);

    run_firtool(fir, &path_in_crate_dir("./test_vec_arr"))?;

    Ok(())
  }
}
