use cmtir::CmtError;
use cmtrs;
use std::path::{Path, PathBuf};

pub enum SimBackend {
  Verilator,
  KSim,
}

pub enum Backend {
  Sv,
  Fir,
  CmtIR,
  Sim(SimBackend),
}

pub struct ElaborateConfig {
  pub path: PathBuf,
  pub backend: Backend,
}

/// Make a config for System Verilog generation
pub fn sv_config(path: impl AsRef<Path>) -> ElaborateConfig {
  ElaborateConfig {
    path: path.as_ref().to_path_buf(),
    backend: Backend::Sv,
  }
}

/// Make a config for FIRRTL generation
///
/// Will generate module into separate files if the path is a directory
pub fn fir_config(path: impl AsRef<Path>) -> ElaborateConfig {
  ElaborateConfig {
    path: path.as_ref().to_path_buf(),
    backend: Backend::Fir,
  }
}

/// Make a config for Cement IR generation
pub fn cmtir_config(path: impl AsRef<Path>) -> ElaborateConfig {
  ElaborateConfig {
    path: path.as_ref().to_path_buf(),
    backend: Backend::CmtIR,
  }
}

/// Make a config for Verilator testbench generation.
///
/// Path should be a directory. Cmake configuration will be generated.
pub fn verilator_config(path: impl AsRef<Path>) -> ElaborateConfig {
  ElaborateConfig {
    backend: Backend::Sim(SimBackend::Verilator),
    path: path.as_ref().to_path_buf(),
  }
}

/// Make a config for Khronos testbench generation.
///
/// Path should be a directory. MakeFile configuration will be generated.
pub fn ksim_config(path: impl AsRef<Path>) -> ElaborateConfig {
  ElaborateConfig {
    backend: Backend::Sim(SimBackend::KSim),
    path: path.as_ref().to_path_buf(),
  }
}

fn elaborate_impl(
  module: impl cmtrs::Instance,
  config: ElaborateConfig,
) -> anyhow::Result<()> {
  match config.backend {
    Backend::Sv => {
      let (_cmtir, firrtl) = crate::to_fir::to_fir_pipeline(module.to_cmtir())?;
      crate::to_fir::run_firtool(firrtl, config.path)
    }
    Backend::Fir => {
      let (_cmtir, firrtl) = crate::to_fir::to_fir_pipeline(module.to_cmtir())?;
      let path = &config.path;
      let target_is_file =
        path.is_file() || path.extension().is_some_and(|e| e == "fir");

      if !target_is_file {
        std::fs::create_dir_all(path)?;
      }
      for (main_name, content) in firrtl {
        let tmp_fir_path = if !target_is_file {
          path.join(&main_name).with_extension("fir")
        } else {
          path.clone()
        };
        crate::utils::print_to(content, Some(&tmp_fir_path));
      }
      Ok(())
    }
    Backend::CmtIR => {
      let (cmtir, _firrtl) = crate::to_fir::to_fir_pipeline(module.to_cmtir())?;
      crate::utils::print_to(cmtir, Some(&config.path));
      Ok(())
    }
    Backend::Sim(backend) => match backend {
      SimBackend::Verilator => {
        crate::sim::verilator::create_verilator_workspace(
          module.to_cmtir(),
          config.path,
        )
      }
      SimBackend::KSim => {
        crate::sim::ksim::create_ksim_workspace(module.to_cmtir(), config.path)
      }
    },
  }
}

/// Elaborate Cmtrs modules into the specified target
/// Available targets:
/// + System Verilog: [`sv_config`]
/// + FIRRTL: [`fir_config`]
/// + CementIR: [`cmtir_config`]
/// + Verilator: [`verilator_config`]
/// + Khronos: [`ksim_config`]
pub fn elaborate(
  module: impl cmtrs::Instance,
  config: ElaborateConfig,
) -> anyhow::Result<()> {
  match elaborate_impl(module, config) {
    Ok(_) => {
      log::info!("Elaboration successful");
    }
    Err(e) => {
      log::error!("Elaboration failed");

      // if e is CmtError, pretty print the error
      match e.downcast_ref::<CmtError>() {
        Some(cmte) => {
          cmte.print();
        }
        None => {
          eprintln!("{}", e);
        }
      }
    }
  }
  Ok(())
}
