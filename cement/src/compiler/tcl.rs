use std::collections::HashMap;

use irony_cmt::{OpId, StructFields};

#[derive(Debug, Clone, Default)]
pub struct TclTable {
  pub tcl_table: HashMap<OpId, TclIP>,
}

impl TclTable {
  pub fn add(&mut self, op_id: OpId, tcl: TclIP) {
    self.tcl_table.insert(op_id, tcl);
  }
}

#[StructFields(pub)]
#[derive(Debug, Clone)]
pub struct TclIP {
  name: String,
  module_name: String,
  vender: String,
  library: String,
  version: String,
  property: HashMap<String, String>,

}

impl TclIP {
  pub fn new_xilinx_ip(
    name: &str,
    module_name: &str,
    version: &str,
    property: HashMap<&str, &str>,
  ) -> TclIP {
    TclIP {
      name: name.to_string(),
      module_name: module_name.to_string(),
      vender: "xilinx.com".to_string(),
      library: "ip".to_string(),
      version: version.to_string(),
      property: property.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }
  }

}