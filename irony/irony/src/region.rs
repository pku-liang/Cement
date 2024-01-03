use super::common::Id;
use super::entity::EntityId;
use super::environ::Environ;
use super::operation::OpId;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Region {
  pub id: usize,
  pub isolated: bool,
  pub op_children: Vec<OpId>,
  pub entity_children: Vec<EntityId>,
}

impl Id for Region {
  fn id(&self) -> usize { self.id }

  fn set_id(&mut self, id: usize) { self.id = id }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(pub usize);
impl Id for RegionId {
  fn id(&self) -> usize { self.0 }

  fn set_id(&mut self, id: usize) { self.0 = id }
}

impl Region {
  pub fn get_use<E: Environ>(&self, env: &E) -> Option<OpId> {
    env.get_region_use(self.as_id())
  }

  pub fn as_id(&self) -> RegionId { RegionId(self.id) }

  pub fn new(isolated: bool) -> Self {
    Self {
      id: 0,
      isolated,
      op_children: vec![],
      entity_children: vec![],
    }
  }

  pub fn add_op_child(&mut self, op: OpId) {
    if let Some(_) = self.op_children.iter().find(|&op_exist| op_exist.id() == op.id()) {
      panic!("{} has already been in the op_children of {}\n", op.id(), self.id())
    } else {
      self.op_children.push(op)
    }
  }

  pub fn delete_op_child(&mut self, op: OpId) {
    if let Some(index) = self.op_children.iter().position(|&op_exist| op_exist.id() == op.id()) {
      self.op_children.remove(index);
    } else {
      panic!("{} is not in the op_children of {}\n", op.id(), self.id())
    }
  }

  pub fn add_entity_child(&mut self, entity: EntityId) {
    if let Some(_) =
      self.entity_children.iter().find(|&entity_exist| entity_exist.id() == entity.id())
    {
      panic!("{} has already been in the entity_children of {}", entity.id(), self.id())
    } else {
      self.entity_children.push(entity)
    }
  }

  pub fn delete_entity_child(&mut self, entity: EntityId) {
    if let Some(index) =
      self.entity_children.iter().position(|&entity_exist| entity_exist.id() == entity.id())
    {
      self.entity_children.remove(index);
    } else {
      panic!("{} is not in the entity_children of {}", entity.id(), self.id())
    }
  }

  pub fn get_op_children(&self) -> Vec<OpId> { self.op_children.to_owned() }

  pub fn get_entity_children(&self) -> Vec<EntityId> { self.entity_children.to_owned() }
}
