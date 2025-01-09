use crate::*;
use cmtir as ir;
use std::path::PathBuf;

pub(crate) fn pre_sim_pipeline(
  mut circuit: ir::Circuit,
) -> anyhow::Result<ir::Circuit> {
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
  let mut set_private_pass = SetPrivatePass::new();
  set_private_pass.apply_pass(&mut circuit)?;
  let mut methodrel_infer_pass = MethodRelInferPass::new();
  methodrel_infer_pass.apply_pass(&mut circuit)?;
  let mut sched_pass = SchedPass::with(inline_pass.modules_to_synthesize);
  sched_pass.apply_pass(&mut circuit)?;

  Ok(circuit)
}

#[allow(unused)]
pub(crate) fn crate_dir() -> PathBuf {
  let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  PathBuf::from(dir)
}

#[test]
fn test_crate_dir() {
  let dir = crate_dir();
  println!("{}", dir.display());
}
