use irony::{Entity, EntityId, Environ, Op};

use crate::{ArrayAttr, AttributeEnum, DataTypeEnum, TypeAttr};

pub fn extract_attrs_for_region<E, EntityT, F, G>(
  env: &E, region_id: irony::RegionId, op_name: &str, f: F, g: G,
) -> Option<AttributeEnum>
where
  E: irony::Environ<EntityT = EntityT>,
  EntityT: Entity<DataTypeT = DataTypeEnum, AttributeT = AttributeEnum>,
  F: Fn(&<E as Environ>::OpT) -> Vec<(String, Vec<Option<irony::EntityId>>)>,
  G: Fn(&E, &irony::EntityId) -> AttributeEnum,
{
  let region = env.get_region(region_id);
  let input_types = region
    .op_children
    .iter()
    .find(|&op_id| {
      let op = env.get_op(*op_id);
      op.get_op_name() == op_name
    })
    .and_then(|x| {
      let op = env.get_op(*x);
      let defs = f(op);
      let inputs = &defs[0].1;
      let input_attrs = inputs
        .into_iter()
        .filter(|&x| x.is_some())
        .map(|x| {
          let Some(x) = x else { panic!() };
          g(env, x)
        })
        .collect::<Vec<AttributeEnum>>();
      Some(input_attrs)
    })
    .unwrap();

  Some(ArrayAttr(input_types).into())
}

pub fn extract_input_names<E, EntityT>(
  env: &E, region_id: irony::RegionId,
) -> Option<AttributeEnum>
where
  E: irony::Environ<EntityT = EntityT>,
  EntityT: Entity<DataTypeT = DataTypeEnum, AttributeT = AttributeEnum>,
{
  extract_attrs_for_region(
    env,
    region_id,
    "HwInput",
    |op: &<E as Environ>::OpT| op.get_defs(),
    |env: &E, x: &EntityId| {
      let name =
        irony::utils::extract_vec(&env.get_entity(*x).get_attrs(), "name").unwrap();
      name
    },
  )
}

pub fn extract_input_types<E, EntityT>(
  env: &E, region_id: irony::RegionId,
) -> Option<AttributeEnum>
where
  E: irony::Environ<EntityT = EntityT>,
  EntityT: Entity<DataTypeT = DataTypeEnum, AttributeT = AttributeEnum>,
{
  extract_attrs_for_region(
    env,
    region_id,
    "HwInput",
    |op: &<E as Environ>::OpT| op.get_defs(),
    |env: &E, x: &EntityId| {
      let input = env.get_entity(*x);
      let type_attr = TypeAttr(input.get_dtype().unwrap());
      AttributeEnum::TypeAttr(type_attr)
    },
  )
}

pub fn extract_output_types<E, EntityT>(
  env: &E, region_id: irony::RegionId,
) -> Option<AttributeEnum>
where
  E: irony::Environ<EntityT = EntityT>,
  EntityT: Entity<DataTypeT = DataTypeEnum, AttributeT = AttributeEnum>,
{
  extract_attrs_for_region(
    env,
    region_id,
    "HwOutput",
    |op: &<E as Environ>::OpT| op.get_uses(),
    |env: &E, x: &EntityId| {
      let input = env.get_entity(*x);
      let type_attr = TypeAttr(input.get_dtype().unwrap());
      AttributeEnum::TypeAttr(type_attr)
    },
  )
}

pub fn extract_types<E, EntityT>(
  env: &E, entities: Vec<Option<irony::EntityId>>,
) -> Option<AttributeEnum>
where
  E: irony::Environ<EntityT = EntityT, AttributeT = AttributeEnum>,
  EntityT: Entity<DataTypeT = DataTypeEnum, AttributeT = AttributeEnum>,
{
  Some(
    ArrayAttr(
      entities
        .into_iter()
        .filter(|x| x.is_some())
        .map(|x| {
          let Some(x) = x else { panic!() };
          let input = env.get_entity(x);
          TypeAttr(input.get_dtype().unwrap()).into()
        })
        .collect::<Vec<AttributeEnum>>(),
    )
    .into(),
  )
}
