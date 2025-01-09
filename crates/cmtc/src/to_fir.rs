use super::*;

mod builder;
pub use builder::*;
mod transform;
pub use transform::*;

use std::{path::Path, process::Command};

/// Compile from original cmtir to firrtl circuits
pub fn to_fir_pipeline(
  mut circuit: ir::Circuit,
) -> Result<(String, Vec<(String, String)>), anyhow::Error> {
  let mut fsmgen_pass = FSMGenPass::new();
  fsmgen_pass.apply_pass(&mut circuit)?;
  let mut passthrough_pass = PassThrough::new();
  passthrough_pass.apply_pass(&mut circuit)?;
  let mut set_synth_pass = SetSynthPass::new();
  set_synth_pass.apply_pass(&mut circuit)?;
  let mut set_private_pass = SetPrivatePass::new();
  set_private_pass.apply_pass(&mut circuit)?;
  let mut methodrel_infer_pass = MethodRelInferPass::new();
  methodrel_infer_pass.apply_pass(&mut circuit)?;
  let mut type_infer_pass = TypeInferPass::new();
  type_infer_pass.apply_pass(&mut circuit)?;
  let mut canonicalize_pass = CanonicalizePass::new();
  canonicalize_pass.apply_pass(&mut circuit)?;
  let mut side_effect_pass = SideEffectPass::new();
  side_effect_pass.apply_pass(&mut circuit)?;
  let mut legal_check_pass = LegalCheckPass::new();
  legal_check_pass.apply_pass(&mut circuit)?;
  let mut msched_pass = MSchedPass::new();
  msched_pass.apply_pass(&mut circuit)?;
  let mut mpart_pass = MPartPass::with(msched_pass);
  mpart_pass.apply_pass(&mut circuit)?;
  let mut delay_resolve_pass = DelayResolvePass::new();
  delay_resolve_pass.apply_pass(&mut circuit)?;
  let mut inline_pass = InlinePass::new();
  inline_pass.apply_pass(&mut circuit)?;
  let mut port_resolve_pass = PortResolvePass::new();
  port_resolve_pass.apply_pass(&mut circuit)?;
  let mut type_infer_pass = TypeInferPass::new();
  type_infer_pass.apply_pass(&mut circuit)?;
  let mut set_private_pass = SetPrivatePass::new();
  set_private_pass.apply_pass(&mut circuit)?;
  let mut methodrel_infer_pass = MethodRelInferPass::new();
  methodrel_infer_pass.apply_pass(&mut circuit)?;
  let mut sched_pass = SchedPass::with(inline_pass.modules_to_synthesize);
  sched_pass.apply_pass(&mut circuit)?;
  let cmtir = circuit.ir_dump();
  // log::debug!("circuit after sched: \n{}", cmtir);
  let mut to_fir_pass = ToFirVisitor::new();
  to_fir_pass.apply_pass(&mut circuit)?;

  Ok((cmtir, to_fir_pass.build()))
}

/// Run firtool to generate SystemVerilog from FIRRTL circuits.
pub fn run_firtool(
  contents: Vec<(String, String)>,
  path: impl AsRef<Path>,
) -> Result<(), anyhow::Error> {
  let path = path.as_ref();
  log::info!(
    "running firtool for [{}], path: {}",
    contents
      .iter()
      .map(|(n, _)| n.clone())
      .collect::<Vec<String>>()
      .join(", "),
    path.display()
  );

  let target_is_file =
    path.is_file() || path.extension().is_some_and(|e| e == "sv");

  if !target_is_file {
    std::fs::create_dir_all(path)?;
  }

  for (main_name, content) in contents {
    // print to a tmp.fir file
    let tmp_fir_path = if !target_is_file {
      path.join(&main_name).with_extension("fir")
    } else {
      path.with_extension("fir")
    };
    let tmp_sv_path = if !target_is_file {
      path.join(&main_name).with_extension("sv")
    } else {
      path.with_extension("sv")
    };

    log::info!(
      "running firtool for {}, sv to {}",
      main_name,
      tmp_sv_path.display()
    );

    utils::print_to(content, Some(&tmp_fir_path));
    // run firtool to generate sv
    let mut child = Command::new("firtool")
      .arg(tmp_fir_path)
      .arg("-o")
      .arg(&tmp_sv_path)
      .arg("-O=release")
      .arg("--disable-all-randomization")
      .spawn()
      .unwrap();
    let exit_stat = child.wait().map_err(|e| {
      anyhow::anyhow!(format!("failed to run firtool for {}: {}", main_name, e))
    })?;
    if !exit_stat.success() {
      anyhow::bail!(format!(
        "firtool failed with exit code for {}: {}",
        main_name,
        exit_stat.code().unwrap()
      ));
    }
  }
  Ok(())
}
