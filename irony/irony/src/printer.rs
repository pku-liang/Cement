use crate::{Entity, EntityId, Environ, RegionId};

pub trait OpPrinterTrait {
  type DataTypeT;
  type AttributeT: Clone + PartialEq + std::fmt::Display;
  fn print<'env, E, EntityT: Entity>(
    &self, env: &'env E, attrs: Vec<(String, Self::AttributeT)>,
    uses: Vec<(String, Vec<Option<EntityId>>)>,
    defs: Vec<(String, Vec<Option<EntityId>>)>, regions: Vec<(String, Vec<Option<RegionId>>)>,
  ) -> String
  where
    E: Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
    EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT>;
}
