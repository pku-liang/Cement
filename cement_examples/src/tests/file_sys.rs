
use std::path::PathBuf;

use cmt::preclude::*;

use super::basics::TopPass;

#[test]
fn test_fs() {
  let mut cmtc = Cmtc::new(config! {
    workspace_dir => PathBuf::from("./build").join(function_dir_path!())
  });
  TopPass::default().top_m(&mut cmtc);
  cmtc.generate_workspace();
}
