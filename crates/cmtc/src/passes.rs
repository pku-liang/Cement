//! The passes for the cmtc compiler

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]

use indexmap::IndexMap;
use location_macros::workspace_dir;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::prelude::StableDiGraph;
use petgraph::visit::EdgeRef;
use slotmap::SecondaryMap;

use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};
use cmtir::utils::{setup_logger, string_match, AutoVec};
use cmtir::{IRDump, OpIO, Print, SlotMap};
use num::Integer;

use std::ops::Sub;

use crate::{IfOp, InstRule, InvokeOp, TimingIntv};

use cmtir as ir;
use cmtir::utils;

use crate::builtin;

mod common;
pub use common::*;

mod test_utils;
pub use test_utils::*;

mod visitor;
pub use visitor::*;

mod set_synth;
pub use set_synth::*;

mod function_inline;
pub use function_inline::*;

mod value_context;
pub use value_context::*;

mod type_infer;
pub use type_infer::*;

mod legal_check;
pub use legal_check::*;

mod methodrel_infer;
pub use methodrel_infer::*;

mod dep_models;
pub use dep_models::*;

mod conflict;
pub use conflict::*;

mod msched;
pub use msched::*;

mod mpart;
pub use mpart::*;

mod delay_resolve;
pub use delay_resolve::*;

mod inline;
pub use inline::*;

mod sched;
pub use sched::*;

mod port_resolve;
pub use port_resolve::*;

mod set_private;
pub use set_private::*;

mod side_effect;
pub use side_effect::*;

mod fsmgen;
pub use fsmgen::*;

mod canonicalize;
pub use canonicalize::*;

mod passthrough;
pub use passthrough::*;

pub fn canonicalize(mut circuit: ir::Circuit) -> Result<ir::Circuit, anyhow::Error> {
  let mut canonicalize_pass = CanonicalizePass::new();
  canonicalize_pass.apply_pass(&mut circuit)?;
  Ok(circuit)
}

pub fn side_effect(mut circuit: ir::Circuit) -> Result<ir::Circuit, anyhow::Error> {
  let mut side_effect_pass = SideEffectPass::new();
  side_effect_pass.apply_pass(&mut circuit)?;
  Ok(circuit)
}

pub fn legal_check(
  mut circuit: ir::Circuit,
) -> Result<ir::Circuit, anyhow::Error> {
  let mut legal_check_pass = LegalCheckPass::new();
  legal_check_pass.apply_pass(&mut circuit)?;
  Ok(circuit)
}

pub fn function_inline(
  mut circuit: ir::Circuit,
) -> Result<ir::Circuit, anyhow::Error> {
  let mut function_inline_pass = FunctionInlinePass::new();
  function_inline_pass.apply_pass(&mut circuit)?;
  Ok(circuit)
}

pub fn type_infer(mut circuit: ir::Circuit) -> Result<ir::Circuit, anyhow::Error> {
  let mut type_infer_pass = TypeInferPass::new();
  type_infer_pass.apply_pass(&mut circuit)?;
  Ok(circuit)
}

pub fn msched(
  mut circuit: ir::Circuit,
) -> Result<(ir::Circuit, MSchedPass), anyhow::Error> {
  let mut msched_pass = MSchedPass::new();
  msched_pass.apply_pass(&mut circuit)?;
  Ok((circuit, msched_pass))
}

pub fn mpart(
  mut circuit: ir::Circuit,
  msched: MSchedPass,
) -> Result<(ir::Circuit, IndexMap<RuleId, Vec<RuleId>>), anyhow::Error> {
  let mut mpart_pass = MPartPass::with(msched);
  mpart_pass.apply_pass(&mut circuit)?;
  Ok((circuit, mpart_pass.rule_derive_table))
}

pub fn all_passes(
  mut circuit: ir::Circuit,
) -> Result<(ir::Circuit, IndexMap<RuleId, Vec<RuleId>>), anyhow::Error> {
  let circuit = type_infer(circuit)?;
  let (circuit, msched) = msched(circuit)?;
  let (circuit, rule_derive_table) = mpart(circuit, msched)?;
  Ok((circuit, rule_derive_table))
}
