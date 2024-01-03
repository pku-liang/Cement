use irony_cmt::{CmtIR, Environ, FxHashSet, FxIndexMap, Id, RegionId};

#[derive(Debug, Default)]
pub struct SymbolTable {
  table: FxIndexMap<usize, FxHashSet<String>>,
}

impl SymbolTable {
  pub fn get_legal_name_in_region(&mut self, ir: &CmtIR, raw_name: &str) -> String {
    // let region_code = self.env.parent_stack.last().unwrap().encode();
    let region_code = ir
      .parent_stack
      .iter()
      .rev()
      .find_map(|x| match x {
        Some(x) => {
          if ir.get_region(*x).isolated {
            Some(Some(*x))
          } else {
            None
          }
        },
        None => Some(None),
      })
      .unwrap()
      .encode();

    let raw_name = if raw_name.len() > 50 {
      let mut total_length = 0;
      raw_name
        .split("_")
        .take_while(|x| {
          total_length += x.len() + 1;
          total_length <= 50
        })
        .collect::<Vec<_>>()
        .join("_")
        + "_etc"
    } else {
      raw_name.to_string()
    };

    let retval = {
      match self.table.get_mut(&region_code) {
        Some(name_set) => {
          if name_set.contains(&raw_name) {
            let mut new_name = raw_name.to_owned();
            let mut i = 0;
            while name_set.contains(&new_name) {
              i += 1;
              new_name = format!("{}_{}", raw_name.to_owned(), i);
            }
            name_set.insert(new_name.to_owned());
            new_name
          } else {
            name_set.insert(raw_name.to_owned());
            raw_name.to_owned()
          }
        },
        None => {
          let mut new_name_set = FxHashSet::default();
          new_name_set.insert(raw_name.to_owned());
          self.table.insert(region_code, new_name_set);
          raw_name.to_owned()
        },
      }
    };

    retval
  }
}

trait Encode {
  fn encode(&self) -> usize;
}

impl Encode for Option<RegionId> {
  fn encode(&self) -> usize {
    match self {
      Some(x) => x.id() + 1,
      None => 0,
    }
  }
}
