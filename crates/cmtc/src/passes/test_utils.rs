use std::process::Command;

use location_macros::crate_dir;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use super::*;

pub fn path_in_workspace_dir(path: impl AsRef<Path>) -> PathBuf {
  PathBuf::from(workspace_dir!()).join(path.as_ref())
}

pub fn path_in_crate_dir(path: impl AsRef<Path>) -> PathBuf {
  PathBuf::from(crate_dir!()).join(path.as_ref())
}

#[macro_export]
macro_rules! circuit_from_file {
  ($file:expr) => {{
    make_prepare($file);
    let dir = path_in_workspace_dir($file);
    let str = std::fs::read_to_string(dir.as_path()).expect(
      format!(
        "failed to read {}, run `make cmtirs` first to generate the cmtir file",
        dir.to_str().unwrap()
      )
      .as_str(),
    );
    str.into()
  }};
}