use std::fs::{self, File};
use std::io::Write;
use std::panic::Location;
use std::path::{absolute, PathBuf};

use irony_cmt::{
  Assign, CmtIR, EntityEnum, EntityId, Environ, HwInput, HwInstance, HwModule, HwOutput,
  OpEnum, OpId, PassEnum, PassManagerTrait, Region, RegionId, RemoveEventPass,
  RemoveSelectPass, RemoveUnaryPass, ReorderPass,
};

use crate::gir;
use crate::hcl::Interface;

mod module_stack;
use module_stack::*;

mod symbol_table;
use symbol_table::*;

mod tcl;
pub use tcl::*;

mod basics;
pub use basics::*;

mod event;
pub use event::*;

mod config;
pub use config::*;

mod stmt;
use futures::Future;
pub use stmt::*;

use super::simulator::{SimCoroInterface, Simulator};

pub struct Cmtc {
  pub ir: CmtIR,

  pub config: CmtcConfig,

  pub symbol_table: SymbolTable,
  pub module_stack: ModuleStack,

  pub ip_tcls: TclTable,
}

impl Cmtc {
  pub fn new(config: CmtcConfig) -> Self {
    Cmtc {
      ir: CmtIR::new(),
      config,
      symbol_table: SymbolTable::default(),
      module_stack: ModuleStack::default(),
      ip_tcls: TclTable::default(),
    }
  }

  pub fn module_op_id_iter(&self) -> impl Iterator<Item = OpId> + '_ {
    self.ir.get_ops_with_parent(None).to_owned().into_iter().filter_map(
      |op_id| match self.ir.get_op(op_id) {
        OpEnum::HwModule(_) => Some(op_id),
        _ => None,
      },
    )
  }

  pub fn get_current_module_ip(&self) -> Option<OpId> {
    self.module_stack.current_module()
  }

  fn run_reorder_passes(&mut self) {
    let mut pass_manager = irony_cmt::PassManager::default();
    let start_ops = self.module_op_id_iter().collect::<Vec<_>>();
    pass_manager.add_passes(vec![PassEnum::ReorderPass(ReorderPass)], vec![start_ops]);
    pass_manager.run_passes(&mut self.ir).expect("must run passes successfully");
  }

  fn run_passes(&mut self, passes: Vec<PassEnum>, start_ops: Vec<Vec<OpId>>) {
    let mut pass_manager = irony_cmt::PassManager::default();
    pass_manager.add_passes(passes, start_ops);
    pass_manager.run_passes(&mut self.ir).expect("must run passes successfully");
  }

  pub fn print(&mut self) {
    self.run_reorder_passes();
    self
      .module_op_id_iter()
      .for_each(|module_op_id| println!("{}", self.ir.print_op(module_op_id)));
  }

  pub fn print_with_passes(&mut self, passes: Vec<PassEnum>, start_ops: Vec<Vec<OpId>>) {
    self.run_passes(passes, start_ops);
    self
      .module_op_id_iter()
      .for_each(|module_op_id| println!("{}", self.ir.print_op(module_op_id)));
  }

  pub fn print_common(&mut self) {
    self.run_passes(
      vec![ReorderPass.into(), RemoveEventPass.into(), RemoveSelectPass.into()],
      vec![
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
      ],
    );
    self
      .module_op_id_iter()
      .for_each(|module_op_id| println!("{}", self.ir.print_op(module_op_id)));
  }

  pub fn elaborate(&mut self) {
    self.run_gir_passes();

    self.run_passes(
      vec![
        ReorderPass.into(),
        RemoveEventPass.into(),
        RemoveSelectPass.into(),
        RemoveUnaryPass.into(),
      ],
      vec![
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
      ],
    );
  }

  fn clean_workspace(&mut self) {
    let workspace_dir = self.config.workspace_path();
    match fs::remove_dir_all(workspace_dir.to_owned()) {
      Ok(_) => {
        println!("clean workspace {}", workspace_dir.to_str().unwrap());
      },
      Err(err) => {
        println!("clean workspace failed: {}", err)
      },
    }
  }

  pub fn print_to_file(&mut self) -> PathBuf {
    self.run_passes(
      vec![
        ReorderPass.into(),
        RemoveEventPass.into(),
        RemoveSelectPass.into(),
        RemoveUnaryPass.into(),
      ],
      vec![
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
        self.module_op_id_iter().collect(),
      ],
    );
    let path_dir = self.config.workspace_path();
    fs::create_dir_all(path_dir.to_owned()).expect("must create the target directory");

    println!(
      "generate circt code to {}",
      absolute(path_dir.to_owned()).expect("convert absolute path").to_str().unwrap()
    );

    let file_path =
      absolute(path_dir.to_owned().join("modules.mlir")).expect("convert absolute path");
    let mut file =
      File::create(file_path.to_owned()).expect("must create target file for modules");
    for module_op_id in self.module_op_id_iter() {
      writeln!(file, "{}", self.ir.print_op(module_op_id))
        .expect("must write to target file");
    }
    file_path
  }

  fn generate_verilog_to_files(&mut self) {
    let mlir_file_path = self.print_to_file();
    let file_dir = mlir_file_path.parent().unwrap();
    let mut command = std::process::Command::new(self.config.circt_opt.to_owned());

    let lower_seq = "--lower-seq-to-sv";
    let export_verilog =
      format!("--export-split-verilog=dir-name={}", file_dir.to_str().unwrap());

    command.arg(lower_seq).arg(export_verilog).arg(mlir_file_path.to_str().unwrap());

    println!("{:?}", command);
    let output = command.output().expect("must run circt-opt");
    if !output.status.success() {
      panic!("circt-opt failed");
    }
  }

  fn generate_ip_tcl(&mut self) {
    let ip_tcl_file_dir = self.config.workspace_path().join("make_ip.tcl");
    let mut file = File::create(ip_tcl_file_dir.to_owned())
      .expect("must create target file for ip tcl");
    writeln!(file, "set_part {}", self.config.part).expect("write set_part to tcl");

    for (_module_op_id, tcl_ip) in self.ip_tcls.tcl_table.iter() {
      let TclIP {
        name: ip_name,
        module_name,
        vender: ip_vender,
        library: ip_library,
        version: ip_version,
        property: mut ip_property,
      } = tcl_ip.to_owned();

      ip_property.insert("Component_Name".to_string(), module_name.to_owned());
      let ip_property_str = ip_property
        .iter()
        .map(|(k, v)| format!("Config.{} {{{}}}", k, v))
        .collect::<Vec<_>>()
        .join(" ");
      writeln!(
        file,
        "create_ip -name {} -vendor {} -library {} -version {} -module_name {} -dir {} -force",
        ip_name,
        ip_vender,
        ip_library,
        ip_version,
        module_name,
        self.config.ip_sub_dir.to_str().unwrap()
      )
      .expect("write create_ip to tcl");
      writeln!(
        file,
        "set_property -dict [list {}] [get_ips {}]",
        ip_property_str, module_name,
      )
      .expect("write set_property to tcl");
    }
    writeln!(file, "generate_target all [get_ips]")
      .expect("write generate_target to tcl");
    writeln!(file, "synth_ip [get_ips]").expect("write synth_ip to tcl");
  }

  fn generate_other_tcl(&mut self) {
    // TODO: Fill this!
  }

  pub fn generate_workspace(&mut self) {
    self.clean_workspace();
    self.generate_verilog_to_files();
    self.generate_ip_tcl();
    self.generate_other_tcl();
  }

  pub fn run_gir_passes(&mut self) {
    let graph = gir::passes::all_passes(self);
    gir::passes::retrieve_cmtc(self, graph);
  }

  pub fn simulate<FuncT, FutureT>(&mut self, test_func: FuncT)
  where
    FuncT: FnOnce(SimCoroInterface) -> FutureT,
    FutureT: Future<Output = ()> + Send + 'static,
  {
    self.elaborate();
    Simulator::new(self).test(test_func)
  }
}
