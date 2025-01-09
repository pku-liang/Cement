use super::{verilator, *};
use cmtc::{run_firtool, to_fir_pipeline, Visitor};
use cmtir::utils::print_to;
use std::path::{Path, PathBuf};
use test_utils::pre_sim_pipeline;

mod cpp;
mod instance;

pub struct SimKhronos {
  circuit: ir::Circuit,
  sim_data: interface::SimData,
}

impl SimKhronos {
  pub fn new(circuit: ir::Circuit, sim_data: interface::SimData) -> Self {
    Self { circuit, sim_data }
  }

  pub fn build_verilator_run(&self) -> anyhow::Result<verilator::VerilatorRun> {
    let sim_verilator =
      verilator::SimVerilator::new(self.circuit.clone(), self.sim_data.clone());
    sim_verilator.build_verilator_run()
  }

  pub fn elaborate(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
    log::info!("Elaborating khronos run");
    let verilator_run = self.build_verilator_run()?;
    let ksim_cpp = verilator_run.build_ksim()?;

    let ksim_tb = ksim_cpp.to_cpp();
    let ksim_tb_path = path.as_ref().join("tb.cpp");
    print_to(ksim_tb, Some(&ksim_tb_path));

    Ok(())
  }
}

pub fn create_ksim_workspace(
  circuit: ir::Circuit,
  path: impl AsRef<Path>,
) -> anyhow::Result<()> {
  let path = path.as_ref();
  std::fs::create_dir_all(path)?;

  let design_file_dir = path.join("hw");
  std::fs::create_dir_all(design_file_dir.clone())?;

  let (_cmtir, fir) = to_fir_pipeline(circuit.clone())?;
  // println!("{}", cmtir);
  run_firtool(fir, design_file_dir)?;

  let mut circuit = pre_sim_pipeline(circuit)?;
  let mut sim_itfc = SimItfc::new();
  sim_itfc.apply_pass(&mut circuit)?;
  let sim_khronos = SimKhronos::new(circuit, sim_itfc.into_inner());
  sim_khronos.elaborate(path)?;

  // cp $(crate)/resources/fir-backport.py to path/fir-backport.py

  let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let resources_path = crate_path.join("src/sim/resources");
  let fir_backport_path = resources_path.join("fir-backport.py");
  std::fs::copy(fir_backport_path, path.join("fir-backport.py")).map_err(
    |e| {
      anyhow::anyhow!(
        "failed to copy fir-backport.py to {}: {}",
        path.display(),
        e
      )
    },
  )?;

  // cp $(crate)/resources/Makefile to path/Makefile
  let makefile_path = resources_path.join("Makefile");
  std::fs::copy(makefile_path, path.join("Makefile")).map_err(|e| {
    anyhow::anyhow!("failed to copy Makefile to {}: {}", path.display(), e)
  })?;

  Ok(())
}
