use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};

use irony_cmt::CmtIR;

use crate::compiler::Cmtc;

mod executor;
use executor::*;

mod schedule;
use schedule::*;

mod builder;
use builder::*;

mod state;
pub use state::StateData;
use state::*;

use self::events::PokeEvent;

mod events;

pub struct Simulator {
  container: Arc<RwLock<SimStateContainer>>,
  io_table: Arc<HashMap<String, StateId>>,
  cycle: Arc<RwLock<SimCycle>>,
}

impl Simulator {
  pub fn new(dut: &Cmtc) -> Self {
    let container = Arc::new(RwLock::new(SimStateContainer::new()));

    let top = get_top(dut);

    let (cycle, inputs, outputs) = make_simulator(dut, top, &container);

    let io_table = get_io_name(&dut.ir, top, inputs, outputs);

    Simulator {
      container,
      io_table: Arc::new(io_table),
      cycle: Arc::new(RwLock::new(cycle)),
    }
  }

  pub fn test<FuncT, FutureT>(&self, test_func: FuncT)
  where
    FuncT: FnOnce(SimCoroInterface) -> FutureT,
    FutureT: Future<Output = ()> + Send + 'static,
  {
    let (spawner, executor) = spawn_and_execute(256);

    let interface = SimCoroInterface {
      container: Arc::clone(&self.container),
      io_table: Arc::clone(&self.io_table),
      cycle: Arc::clone(&self.cycle),
      spawner: spawner.clone(),
      barrier: SimBarrier::new(Arc::clone(&self.cycle)),
    };
    interface.active();
    let future = test_func(interface);

    spawner.push(future);

    drop(spawner);
    executor.run();
  }
}

pub struct SimCoroInterface {
  container: Arc<RwLock<SimStateContainer>>,
  io_table: Arc<HashMap<String, StateId>>,
  cycle: Arc<RwLock<SimCycle>>,
  spawner: SimSpawner,
  barrier: Arc<RwLock<SimBarrier>>,
}

impl SimCoroInterface {
  /// Put data into an io port, last for one cycle
  pub fn poke(&self, io_name: &str, data: StateData) {
    self.cycle.write().unwrap().poke_events.push(Box::new(PokeEvent {
      container: Arc::clone(&self.container),
      id: self
        .io_table
        .get(io_name)
        .expect(&format!("Poke: io {} not found", io_name))
        .clone(),
      data,
    }));
  }

  /// Put data into an io port, last until being covered
  pub fn keep_poke(&self, io_name: &str, data: StateData) {
    self.cycle.write().unwrap().keep_poke_events.push(Box::new(PokeEvent {
      container: Arc::clone(&self.container),
      id: self
        .io_table
        .get(io_name)
        .expect(&format!("Poke: io {} not found", io_name))
        .clone(),
      data,
    }));
  }

  /// Get data from an io port
  pub fn peek(&self, io_name: &str) -> StateData {
    self
      .container
      .read()
      .unwrap()
      .read(
        *self.io_table.get(io_name).expect(&format!("Poke: io {} not found", io_name)),
      )
      .clone()
  }

  // /// Get data from an io port, returns a future
  // pub fn peek(&self, io_name: &str) -> SimPeek {
  //   let ready = Arc::new(RwLock::new(false));
  //   self
  //     .cycle
  //     .write()
  //     .unwrap()
  //     .peek_events
  //     .push(Box::new(PeekEvent { ready: Arc::clone(&ready) }));
  //   SimPeek {
  //     containter: Arc::clone(&self.container),
  //     handle: self.io_table.get(io_name).unwrap().clone(),
  //     ready,
  //   }
  // }

  /// Fork a coroutine
  /// Example:
  /// ```
  ///   interface
  ///     .fork(async { ... })
  ///     .fork(async { ... })
  ///     .join().await;
  /// ```
  pub fn fork<FuncT, FutureT>(&self, test_func: FuncT) -> SimJoinInterface
  where
    FuncT: FnOnce(SimCoroInterface) -> FutureT,
    FutureT: Future<Output = ()> + Send + 'static,
  {
    self.make_fork(test_func, Arc::new(RwLock::new(SimTaskJoiner::new())))
  }

  // TODO: specify clock
  /// Forward the clock by 1 cycle
  /// Example: `interface.step().await`
  pub async fn step(&self) { SimBarrier::arrive(&self.barrier).await }

  // TODO: specify clock
  /// Forward the clock by n cycle
  /// Example: `interface.step(3).await`
  pub async fn step_n(&self, n: usize) {
    for _ in 0..n {
      SimBarrier::arrive(&self.barrier).await
    }
  }

  fn make_fork<FuncT, FutureT>(
    &self, test_func: FuncT, sim_join: Arc<RwLock<SimTaskJoiner>>,
  ) -> SimJoinInterface
  where
    FuncT: FnOnce(SimCoroInterface) -> FutureT,
    FutureT: Future<Output = ()> + Send + 'static,
  {
    let cloned = self.clone();
    cloned.active();
    let future = test_func(cloned);

    let interface = SimJoinInterface { sim_interface: self.clone(), sim_join };

    self.spawner.push_join(future, SimTaskJoiner::add(&interface.sim_join));

    interface
  }

  fn active(&self) { self.barrier.write().unwrap().increase(); }

  fn deactive(&self) { self.barrier.write().unwrap().decrease(); }
}

impl Clone for SimCoroInterface {
  fn clone(&self) -> Self {
    SimCoroInterface {
      container: Arc::clone(&self.container),
      io_table: Arc::clone(&self.io_table),
      cycle: Arc::clone(&self.cycle),
      spawner: self.spawner.clone(),
      barrier: Arc::clone(&self.barrier),
    }
  }
}

pub struct SimJoinInterface {
  sim_interface: SimCoroInterface,
  sim_join: Arc<RwLock<SimTaskJoiner>>,
}

impl SimJoinInterface {
  pub fn fork<FuncT, FutureT>(&self, test_func: FuncT) -> SimJoinInterface
  where
    FuncT: FnOnce(SimCoroInterface) -> FutureT,
    FutureT: Future<Output = ()> + Send + 'static,
  {
    self.sim_interface.make_fork(test_func, Arc::clone(&self.sim_join))
  }

  pub async fn join(&self) {
    self.sim_interface.deactive();
    let future = SimTaskJoiner::wait(&self.sim_join);
    future.await;
    self.sim_interface.active();
  }
}
