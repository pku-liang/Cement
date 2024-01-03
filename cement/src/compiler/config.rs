use std::path::{Path, PathBuf};

use crate::preclude::CfgXilinxIP;

pub enum CfgValue {
  Bool(bool),
  String(String),
  PathBuf(PathBuf),
  XilinxIp(CfgXilinxIP),
}

impl From<bool> for CfgValue {
  fn from(value: bool) -> Self { CfgValue::Bool(value) }
}

impl From<String> for CfgValue {
  fn from(value: String) -> Self { CfgValue::String(value) }
}

impl From<&str> for CfgValue {
  fn from(value: &str) -> Self { CfgValue::String(value.to_string()) }
}

impl From<PathBuf> for CfgValue {
  fn from(value: PathBuf) -> Self { CfgValue::PathBuf(value) }
}

pub struct CmtcConfig {
  pub deduplicate: bool,
  pub debug: bool,
  pub onehot: bool,
  pub circt_opt: PathBuf,
  workspace_dir: PathBuf,
  workspace_name: String,
  pub ip_sub_dir: PathBuf,
  pub part: String,
  pub xilinx_ip_config: CfgXilinxIP,
  // TODO: add more board-/synthesis-/implementation-related configurations
}

static CIRCT_PATH: &'static str = "/home/uvxiao/repos/circt/build/bin";

impl Default for CmtcConfig {
  fn default() -> Self {
    let circt_path = std::env::var("CIRCT_PATH").unwrap_or_else({
      println!("CIRCT_PATH not set, using default");
      |_| CIRCT_PATH.to_string()
    });
    CmtcConfig {
      deduplicate: true,
      debug: false,
      onehot: false,
      circt_opt: PathBuf::from(circt_path).join("circt-opt"),
      workspace_dir: Path::new("./build").to_path_buf(),
      workspace_name: "ws".to_string(),
      ip_sub_dir: Path::new("./ip").to_path_buf(),
      part: "xcu200-fsgd2104-2-e".to_string(),
      xilinx_ip_config: CfgXilinxIP { latency_read: 1, latency_fixed_add: 2 },
    }
  }
}

impl CmtcConfig {
  pub fn workspace_path(&self) -> PathBuf {
    self.workspace_dir.join(&self.workspace_name)
  }

  pub fn with_subdir(subdir: PathBuf) -> Self {
    CmtcConfig {
      workspace_dir: Path::new("./build").join(subdir),
      ..Default::default()
    }
  }

  pub fn from_dict(dict: Vec<(String, CfgValue)>) -> Self {
    let mut config = CmtcConfig::default();
    for (key, value) in dict {
      match key.as_str() {
        "deduplicate" => {
          if let CfgValue::Bool(b) = value {
            config.deduplicate = b;
          }
        },
        "debug" => {
          if let CfgValue::Bool(b) = value {
            config.debug = b;
          }
        },
        "onehot" => {
          if let CfgValue::Bool(b) = value {
            config.onehot = b;
          }
        },
        "circt_opt" => {
          if let CfgValue::String(s) = value {
            config.circt_opt = PathBuf::from(s);
          }
        },
        "workspace_dir" => match value {
          CfgValue::String(s) => {
            config.workspace_dir = PathBuf::from(s);
          },
          CfgValue::PathBuf(p) => {
            config.workspace_dir = p;
          },
          _ => panic!("workspace_dir must be a string or a PathBuf"),
        },
        "workspace_name" => {
          if let CfgValue::String(s) = value {
            config.workspace_name = s;
          }
        },
        "ip_sub_dir" => {
          if let CfgValue::String(s) = value {
            config.ip_sub_dir = PathBuf::from(s);
          }
        },
        "part" => {
          if let CfgValue::String(s) = value {
            config.part = s;
          }
        },
        "xilinx_ip_config" => {
          if let CfgValue::XilinxIp(ip_config) = value {
            config.xilinx_ip_config = ip_config;
          }
        },
        _ => panic!("unknown config key: {}", key),
      }
    }
    config
  }
}

impl Into<CmtcConfig> for Vec<(String, CfgValue)> {
  fn into(self) -> CmtcConfig { CmtcConfig::from_dict(self) }
}

#[macro_export]
macro_rules! config {
    ($($k:ident => $v:expr),* $(,)?) => {
        CmtcConfig::from_dict(vec![$((stringify!($k).to_string(), Into::<CfgValue>::into($v)),)*])
    };
}

#[macro_export]
macro_rules! function_dir {
  () => {{
    fn f() {}
    fn type_name_of<T>(_: T) -> &'static str { std::any::type_name::<T>() }
    let name = type_name_of(f);
    name.strip_suffix("::f").unwrap()
  }};
}

pub fn function_dir_as_path(dir_str: &str) -> PathBuf {
  dir_str.to_string().split("::").fold(PathBuf::new(), |path, str| path.join(str))
}

#[macro_export]
macro_rules! function_dir_path {
  () => {
    function_dir_as_path(function_dir!())
  };
}
