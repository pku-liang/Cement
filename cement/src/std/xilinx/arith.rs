use crate::preclude::*; 
#[interface]
pub struct FixedAdd<T: DataTypeTrait> {
  pub A: T,
  pub B: T,
  pub S: <T as Interface>::FlipT,
}

impl<T: DataTypeTrait> FixedAdd<T> {
  pub fn new(data_type: T) -> Self {
    Self {
      A: data_type.clone(),
      B: data_type.clone(),
      S: data_type.flip(),
    }
  }
}

module_ext! {
  <T: DataTypeTrait> FixedAdd<T> =>
  add_fixed(io, latency: u32)[
    tcl = TclIP::new_xilinx_ip(
      "c_addsub", "add_fixed", "12.0", 
      [
        ("Implementation", "DSP48"), 
        ("A_Width", &format!("{}", io.A.data_type().width())),
        ("B_Width", &format!("{}", io.B.data_type().width())),
        ("Out_Width", &format!("{}", io.S.data_type().width())),
        ("Latency", &format!("{}", latency)),
        ("B_Value", "0000000000000000"),
        ("CE", "false")
      ].into()
    )
  ] {}
}

