use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use crossbeam::epoch;
use crossbeam::deque;
use crate::core::task::*;
use crate::utils::thread_pinning::AFFINITY_MAPPING;

pub struct Workers<'a> {
  is_finished: &'a AtomicBool,
  worker_count: usize,
  worker: deque::Worker<epoch::Owned<TaskObject>>,
  stealers: &'a [deque::Stealer<epoch::Owned<TaskObject>>],
  activities: &'a [epoch::Atomic<TaskObject>]
}

impl<'a> Workers<'a> {
  pub fn run(worker_count: usize, initial_task: Task) {
    let workers: Vec<deque::Worker<epoch::Owned<TaskObject>>> = (0 .. worker_count).into_iter().map(|_| deque::Worker::new_fifo()).collect();
    let stealers: Box<[deque::Stealer<epoch::Owned<TaskObject>>]> = workers.iter().map(|w| w.stealer()).collect();

    workers[0].push(unsafe { epoch::Owned::from_raw(initial_task.into_raw()) });

    let activities = vec![epoch::Atomic::null(); worker_count].into_boxed_slice();

    let is_finished = AtomicBool::new(false);

    crossbeam::scope(|s| {
      for (thread_index, worker) in workers.into_iter().enumerate() {
        let workers = Workers{
          is_finished: &is_finished,
          worker_count,
          worker,
          stealers: &stealers,
          activities: &activities
        };
        s.spawn(move |_| {
          affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
          workers.do_work(thread_index);
        });
      }
    }).unwrap();
  }

  pub fn finish(&self) {
    self.is_finished.store(true, Ordering::Release);
  }

  pub fn push_task(&self, task: Task) {
    self.worker.push(unsafe { epoch::Owned::from_raw(task.into_raw()) });
  }

  fn do_work(&self, thread_index: usize) {
    loop {
      if self.is_finished.load(Ordering::Relaxed) {
        return;
      }

      // First try work stealing of tasks, to exploit task parallelism.
      if let Some(task) = self.claim_task(thread_index) {
        self.start_task(task, thread_index);
      } else {
        // There is not enough task parallelism.
        // We try to perform work assisting on data parallel workloads.
        self.try_assist(thread_index);
      }
    }
  }

  fn claim_task(&self, thread_index: usize) -> Option<epoch::Owned<TaskObject>> {
    // First we try to claim a task from our own deque.
    if let Some(item) = self.worker.pop() {
      return Some(item);
    }
    // If we didn't have tasks on our own deque, we try to steal a task from another thread.
    let mut other_index = thread_index;
    let increment = if thread_index % 2 == 0 { 1 } else { self.worker_count - 1 };
    loop {
      other_index = (other_index + increment) % self.worker_count;
      if other_index == thread_index {
        break;
      }
      if let Some(item) = self.stealers[other_index].steal().success() {
        return Some(item);
      }
    }
    None
  }

  fn try_assist(&self, thread_index: usize) {
    let mut other_index = thread_index;
    let increment = if thread_index % 2 == 0 { 1 } else { self.worker_count - 1 };

    let guard = epoch::pin();
    loop {
      other_index = (other_index + increment) % self.worker_count;
      if other_index == thread_index {
        break;
      }

      let activity = self.activities[other_index].load(Ordering::Acquire, &guard);
      if activity.is_null() { continue; }

      // We can assist this thread
      // Note that `task` gets a different lifetime than `guard` and `activitity`.
      // This is safe as the reference count will assure that the object stays live.
      let task = unsafe { &*activity.as_raw() };
      let mut signal = EmptySignal{ pointer: &self.activities[other_index], old: task, state: EmptySignalState::Assist };

      // Mark that this thread is working on the task and claim the first chunk
      let (old_active_threads, current_index) = task.counters.fetch_add(1, 1, Ordering::Acquire);

      if old_active_threads == 0 {
        // This work was already finished.
        signal.task_empty();
        // We don't call end_task() now, as another thread will call or has called the finish function.
        // end_task would decrement active_threads again, which we do not want as it will now never reach
        // 0 any more.
        break;
      }

      drop(guard);

      // Early out.
      if current_index >= task.work_size {
        signal.task_empty();
        self.end_task(task);
        break;
      }
      self.call_task(task, signal, current_index);
      break;
    }
  }

  fn start_task(&self, task_owned: epoch::Owned<TaskObject>, thread_index: usize) {
    let task_shared = task_owned.into_shared(unsafe { epoch::unprotected() });
    let task = unsafe { task_shared.deref() };

    if task.work_size == 0 {
      // This task doesn't have data parallelism.
      // Hence task.function doesn't need to be called,
      // only task.continuation.
      (task.continuation)(self, unsafe { &*Task::ptr_data(task_shared.as_raw()) });
      // Safety: no other threads will work on this task,
      // as it is never pushed to the 'activities' list.
      // Hence we can take unique ownership of this task here,
      // and drop it.
      drop(unsafe { Task::from_raw(task_shared.as_raw() as *mut TaskObject) });
      return;
    }

    self.activities[thread_index].store(task_shared, Ordering::Release);

    let signal = EmptySignal{ pointer: &self.activities[thread_index], old: task, state: EmptySignalState::Main };
    self.call_task(task, signal, 0);
  }

  // Calls the work function of a task, and calls end_task afterwards
  fn call_task(&self, task: &TaskObject, signal: EmptySignal, first_index: u32) {
    (task.function)(self, unsafe { &*Task::ptr_data(&*task) }, LoopArguments{ work_size: task.work_size, work_index: task.counters.work_index(), empty_signal: signal, first_index });
    self.end_task(task);
  }

  fn end_task(&self, task: &TaskObject) {
    // Check whether there is no pending work (that is claimed, but not finished yet).
    let remaining = task.counters.active_threads().fetch_sub(1, Ordering::AcqRel) - 1;
    if remaining == 0 {
      // Only one thread will decrement active_threads to zero.
      // That thread will call the continuation of the task.
      (task.continuation)(self, unsafe { &*Task::ptr_data(&*task) });
      let guard = epoch::pin();
      // At the end of this epoch, we can take unique ownership of the TaskObject
      let task = unsafe { Task::from_raw(&*task as *const TaskObject as *mut TaskObject) };
      guard.defer(move || drop(task));
    }
  }
}

pub struct EmptySignal<'a> {
  pointer: &'a epoch::Atomic<TaskObject>,
  old: *const TaskObject,
  state: EmptySignalState
}

enum EmptySignalState {
  Main,
  Assist,
  DidSignal
}

impl<'a> EmptySignal<'a> {
  pub fn task_empty(&mut self) {
    match self.state {
      EmptySignalState::DidSignal => {},
      EmptySignalState::Main => {
        self.pointer.store(epoch::Shared::null(), Ordering::Relaxed);
      },
      EmptySignalState::Assist => {
        self.pointer.compare_exchange(epoch::Shared::from(self.old), epoch::Shared::null(), Ordering::Relaxed, Ordering::Relaxed, unsafe { epoch::unprotected() }).ok();
        self.state = EmptySignalState::DidSignal;
      }
    }
  }
}
