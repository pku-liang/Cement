use super::*;

impl VerilatorRun {
  pub fn to_cmake(&self) -> String {
    let mut s = String::new();
    s.push_str(&format!(
      "
cmake_minimum_required(VERSION 3.24)
project({})
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED True)
find_package(verilator HINTS $ENV{{VERILATOR_ROOT}})

add_executable(Vsim tb.cpp)

",
      self.name
    ));

      s.push_str(&format!(
        "
set(DESIGNDIR ./hw)

{}
",
        self
          .instances
          .iter()
          .map(|i| i.to_impl(&self.circuit).to_cmake())
          .collect::<Vec<_>>()
          .join("\n")
      ));
    
    s
  }

  // pub fn build_cmake(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
  //   let path = path.as_ref().join("CmakeLists.txt");
  //   print_to(self.to_cmake(), Some(&path));
  //   Ok(())
  // }
}
