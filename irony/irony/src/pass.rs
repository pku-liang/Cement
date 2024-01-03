use crate::{Environ, OpId};

pub trait PassTrait<T: Default, ERR>: Clone {
  type EntityT;
  type OpT;
  // TODO: future features
  // fn get_statistics_mut(&self) -> &mut PassStatistics;
  // fn get_arguments_str() -> String;
  // fn get_name_str() -> String;
  // fn get_description_str() -> String;

  fn check_op<E>(&self, env: &E, op: OpId) -> bool
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT>;
  fn run_raw<E>(&self, env: &mut E, op: OpId) -> Result<T, ERR>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT>;
  fn run_on<E>(&self, env: &mut E, op: OpId) -> Result<T, ERR>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT> {
    if self.check_op(env, op) {
      self.run_raw(env, op)
    } else {
      Ok(T::default())
    }
  }
}

pub trait PassManagerTrait<T: Default, ERR>: Clone {
  type EntityT;
  type OpT;
  type PassT: PassTrait<T, ERR>;
  fn add_passes(&mut self, passes: Vec<Self::PassT>, start_ops: Vec<Vec<OpId>>);
  fn run_passes<E>(&self, env: &mut E) -> Result<T, ERR>
  where E: Environ<EntityT = Self::EntityT, OpT = Self::OpT>;
}

// TODO: future features
pub struct PassPipeline;
pub struct PassStatistics;

// TODO: Visiters
