use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::task::Poll;

use futures::channel::oneshot;
use futures::future::{BoxFuture, FutureExt};
use futures::task::{waker_ref, ArcWake, Context, Waker};
use futures::Future;
use visible::StructFields;

use super::state::{SimStateContainer, StateData, StateId};
use super::SimCycle;

// macro_rules! sim_task_wrap {
//   ($e:expr) => {{
//     let finish = std::sync::atomic::AtomicBool::new(false);
//     (
//       async {
//         $e.await;
//         finish.store(true, std::sync::atomic::Ordering::Release);
//       },
//       finish,
//     )
//   }};
// }

pub fn spawn_and_execute(fifo_size: usize) -> (SimSpawner, SimExecutor) {
  let (sender, receiver) = sync_channel(fifo_size);
  let spawner = SimSpawner { sender };
  let executor = SimExecutor { tasks: receiver };
  (spawner, executor)
}

#[derive(Clone)]
pub struct SimSpawner {
  sender: SyncSender<Arc<SimTask>>,
}

impl SimSpawner {
  pub fn push(self: &Self, future: impl Future<Output = ()> + 'static + Send) {
    let future = Mutex::new(Some(future.boxed()));

    let task = Arc::new(SimTask { future, sender: self.sender.clone() });
    self.sender.try_send(task).expect("Simulator FIFO fulled!");
  }

  pub fn push_join(
    self: &Self, future: impl Future<Output = ()> + 'static + Send,
    sim_join: &Arc<RwLock<SimTaskJoiner>>,
  ) {
    let cloned = Arc::clone(sim_join);
    let wrapped = async move {
      future.await;
      cloned.write().unwrap().finish();
    };

    self.push(wrapped);
  }

  // pub fn get_sender(&self) -> SyncSender<Arc<SimTask>> { self.sender.clone() }
}

pub struct SimExecutor {
  tasks: Receiver<Arc<SimTask>>,
}

impl SimExecutor {
  pub fn run(self: &Self) {
    while let Ok(task) = self.tasks.recv() {
      let mut future_slot = task.future.lock().unwrap();
      if let Some(mut future) = future_slot.take() {
        let waker = waker_ref(&task);
        let context = &mut Context::from_waker(&waker);
        if future.as_mut().poll(context).is_pending() {
          *future_slot = Some(future);
        }
      }
    }
  }
}

pub struct SimTask {
  future: Mutex<Option<BoxFuture<'static, ()>>>,
  sender: SyncSender<Arc<SimTask>>,
}

impl ArcWake for SimTask {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    let cloned = arc_self.clone();
    arc_self.sender.try_send(cloned).expect("Simulator FIFO fulled!");
  }
}

pub struct SimTaskJoiner {
  target: usize,
  finished: usize,
  waker: Option<Waker>,
}

impl SimTaskJoiner {
  pub fn new() -> Self { SimTaskJoiner { target: 0, finished: 0, waker: None } }

  pub fn add(rw_self: &Arc<RwLock<Self>>) -> &Arc<RwLock<Self>> {
    rw_self.write().unwrap().target += 1;
    rw_self
  }

  fn finish(&mut self) {
    self.finished += 1;
    if self.finished == self.target {
      self.waker.take().unwrap().wake();
    }
  }

  fn is_finished(&self) -> bool { self.finished == self.target }

  pub async fn wait(rw_self: &Arc<RwLock<Self>>) {
    SimTaskJoinerFuture { sim_join: Arc::clone(rw_self) }.await
  }
}

struct SimTaskJoinerFuture {
  sim_join: Arc<RwLock<SimTaskJoiner>>,
}

impl Future for SimTaskJoinerFuture {
  type Output = ();

  fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let finished = self.sim_join.read().unwrap().is_finished();
    if finished {
      self.sim_join.write().unwrap().waker = Some(cx.waker().clone());
      Poll::Pending
    } else {
      Poll::Ready(())
    }
  }
}

pub struct SimBarrier {
  target: usize,
  arrived: usize,
  wakers: Vec<Waker>,
  cycle: Arc<RwLock<SimCycle>>,
}

impl SimBarrier {
  pub fn new(cycle: Arc<RwLock<SimCycle>>) -> Arc<RwLock<Self>> {
    Arc::new(RwLock::new(SimBarrier {
      target: 0,
      arrived: 0,
      wakers: Vec::new(),
      cycle,
    }))
  }

  pub fn arrive(barrier: &Arc<RwLock<Self>>) -> SimBarrierFuture {
    SimBarrierFuture { barrier: Arc::clone(barrier) }
  }

  pub fn increase(&mut self) { self.target += 1; }

  pub fn decrease(&mut self) {
    self.target -= 1;
    if self.target == self.arrived {
      self.do_synced();
    }
  }

  fn leave(&mut self, waker: Waker) -> bool {
    if self.target == self.arrived + 1 {
      self.do_synced();
      true
    } else {
      self.wakers.push(waker);
      self.arrived += 1;
      false
    }
  }

  fn do_synced(&mut self) {
    self.arrived = 0;
    for waker in &self.wakers {
      waker.wake_by_ref();
    }
    self.cycle.write().unwrap().run();
  }
}

pub struct SimBarrierFuture {
  barrier: Arc<RwLock<SimBarrier>>,
}

impl Future for SimBarrierFuture {
  type Output = ();

  fn poll(
    self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
  ) -> Poll<Self::Output> {
    let mut barrier = self.barrier.write().unwrap();
    if barrier.leave(cx.waker().clone()) {
      Poll::Ready(())
    } else {
      Poll::Pending
    }
  }
}

// #[StructFields(pub)]
// #[derive(Debug)]
// pub struct SimPeek {
//   containter: Arc<RwLock<SimStateContainer>>,
//   handle: StateHandle,
//   ready: Arc<RwLock<bool>>,
// }

// impl Future for SimPeek {
//   type Output = StateData;

//   fn poll(self: std::pin::Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
//     let ready = self.ready.read().unwrap().to_owned();
//     if ready {
//       Poll::Ready(self.handle.read_from(&self.containter))
//     } else {
//       Poll::Pending
//     }
//   }
// }
