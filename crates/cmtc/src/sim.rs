//! The simulation backends for the cmtc compiler

use cmtir as ir;
use cmtir::*;

use crate as cmtc;

pub mod interface;
use interface::*;
pub mod cmtirust;
pub mod verilator;
pub mod ksim;

mod test_utils;

mod ir_extension;
use ir_extension::*;