#![allow(incomplete_features)]
#![allow(non_snake_case)]
#![feature(associated_type_defaults)]
#![feature(generic_const_exprs)]
#![feature(macro_metavar_expr)]
#![feature(absolute_path)]

mod compiler;
mod gir;
mod hcl;
pub mod simulator;
mod std;
mod utils;

pub mod preclude;
