use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use crossbeam::deque;
use crate::core::task::*;
use crate::utils::ptr::AtomicTaggedPtr;
use crate::utils::ptr::TaggedPtr;
use crate::utils::thread_pinning::AFFINITY_MAPPING;

pub struct Workers<'a> {
  is_finished: &'a AtomicBool,
  worker_count: usize,
  worker: deque::Worker<Task>,
  stealers: &'a [deque::Stealer<Task>],
  activities: &'a [AtomicTaggedPtr<TaskObject<()>>]
}

impl<'a> Workers<'a> {
  pub fn run(worker_count: usize, initial_task: Task) {
    let workers: Vec<deque::Worker<Task>> = (0 .. worker_count).into_iter().map(|_| deque::Worker::new_lifo()).collect();
    let stealers: Box<[deque::Stealer<Task>]> = workers.iter().map(|w| w.stealer()).collect();

    workers[0].push(initial_task);

    let activities: Box<[AtomicTaggedPtr<TaskObject<()>>]> = unsafe {
      std::mem::transmute(vec![0 as usize; worker_count].into_boxed_slice())
    };

    let is_finished = AtomicBool::new(false);

    std::thread::scope(|s| {
      for (thread_index, worker) in workers.into_iter().enumerate() {
        let workers = Workers{
          is_finished: &is_finished,
          worker_count,
          worker,
          stealers: &stealers,
          activities: &activities
        };
        s.spawn(move || {
          affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
          workers.do_work(thread_index);
        });
      }
    });
  }

  pub fn finish(&self) {
    self.is_finished.store(true, Ordering::Release);
  }

  pub fn push_task(&self, task: Task) {
    self.worker.push(task);
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

  fn claim_task(&self, thread_index: usize) -> Option<Task> {
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

    loop {
      other_index = (other_index + increment) % self.worker_count;
      if other_index == thread_index {
        break;
      }

      let check = self.activities[other_index].load(Ordering::Relaxed);
      if check.ptr().is_null() { continue; }

      // Increment reference count (in tag).
      // Reading in 'check' and in 'activity' may be interleaved, but that is
      // not an issue as we again check whether the point is null.
      // The additional test with 'check' is required, as we could otherwise
      // repeatedly increment the tag of a null pointer, and the tag could then
      // overflow into the bits of the pointer.
      let activity = self.activities[other_index].fetch_add_tag(1, Ordering::Acquire);
      if activity.ptr().is_null() {
        // The before mentioned interleaving happened.
        // We now incremented the tag of a null pointer. We don't have to revert
        // this; as mentioned before this can only happen once per thread so
        // the tag cannot overflow.
        continue;
      }

      // We can assist this thread
      let task = unsafe { &*activity.ptr() };
      let mut signal = EmptySignal{ pointer: &self.activities[other_index], task, state: EmptySignalState::Assist };

      // Claim the first chunk
      let current_index = task.work_index.fetch_add(1, Ordering::Acquire);

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

  fn start_task(&self, task: Task, thread_index: usize) {
    if task.work_size == 0 {
      // This task doesn't have data parallelism.
      // Hence task.function doesn't need to be called,
      // only task.continuation.
      // No other threads will work on this task,
      // as it is never pushed to the 'activities' list.
      // Hence we can take unique ownership of this task here,
      // and pass it to continuation.
      let task_ref: *const TaskObject<()> = &*task;
      let continuation = task.continuation;
      // task.continuation will drop the object. Hence we shouldn't do that here.
      std::mem::forget(task);
      (continuation)(self, task_ref as *mut TaskObject<()>);
      return;
    }

    let task_ptr = task.into_raw();
    let task_ref = unsafe { &*task_ptr };

    // Since this thread previously had no activity (i.e., a null pointer),
    // we don't have to keep track of the reference count that was previously
    // stored in the AtomicTaggedPtr.
    self.activities[thread_index].store(TaggedPtr::new(task_ptr, 1), Ordering::Release);

    let signal = EmptySignal{ pointer: &self.activities[thread_index], task: task_ref, state: EmptySignalState::Main };
    self.call_task(unsafe { &*task_ptr }, signal, 0);
  }

  // Calls the work function of a task, and calls end_task afterwards
  fn call_task(&self, task: *const TaskObject<()>, signal: EmptySignal, first_index: u32) {
    let task_ref = unsafe { &*task };
    (task_ref.function.unwrap())(self, task, LoopArguments{ work_size: task_ref.work_size, work_index: &task_ref.work_index, empty_signal: signal, first_index });
    self.end_task(task);
  }

  fn end_task(&self, task: *const TaskObject<()>) {
    let task_ref = unsafe { &*task };
    // Check whether there is no pending work (that is claimed, but not finished yet).
    let remaining = task_ref.active_threads.fetch_sub(1, Ordering::AcqRel) - 1;
    if remaining == 0 {
      // Only one thread will decrement active_threads to zero.
      // That thread will call the continuation of the task.
      // As documented in TaskObject.active_threads,
      // this task is not present anymore in activities at this point
      // and other threads are not working on this task any more.
      // Hence we can take unique ownership of this task now.
      let continuation = task_ref.continuation;
      // task.continuation will drop the object. Hence we shouldn't do that here.
      (continuation)(self, task as *mut TaskObject<()>);
    }
  }
}

pub struct EmptySignal<'a> {
  pointer: &'a AtomicTaggedPtr<TaskObject<()>>,
  task: &'a TaskObject<()>,
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
        let old = self.pointer.swap(TaggedPtr::new(std::ptr::null(), 0), Ordering::Relaxed);
        // Encorporate the tag of the AtomicTaggedPtr in the reference count of the tag.
        if old.ptr() == self.task {
          self.task.active_threads.fetch_add(old.tag() as i32, Ordering::Relaxed);
          // Note that this fetch-and-add won't decrement the reference count to zero yet,
          // as this thread is working on the task and thus present in the reference count.
          // The reference count will be at least one now. It can only become zero in call_task.
        }
      },
      EmptySignalState::Assist => {
        // self.pointer.compare_exchange(epoch::Shared::from(self.old), epoch::Shared::null(), Ordering::Relaxed, Ordering::Relaxed, unsafe { epoch::unprotected() }).ok();
        // self.state = EmptySignalState::DidSignal;
        let mut value = self.pointer.load(Ordering::Relaxed);
        // Update 'pointer' using a CAS-loop.
        // We must update pointer to not point at 'self.task'.
        // This requires a compare-and-swap (compare-exchange), as we should
        // only update it when it currently points to self.task. It could be
        // that the main thread has progressed to a new task, and overwriting
        // that would prevent any other thread from assisting.
        while value.ptr() == self.task {
          let result = self.pointer.compare_exchange_weak(value, TaggedPtr::new(std::ptr::null(), 0), Ordering::Relaxed, Ordering::Relaxed);
          match result {
            Ok(_) => {
              // Encorporate the tag of the AtomicTaggedPtr in the reference count of the task.
              self.task.active_threads.fetch_add(value.tag() as i32, Ordering::Relaxed);
            },
            Err(new) => {
              // compare-exchange failed. This is caused by either:
              // - Another thread updated the pointer to point to null or a different task. The loop will stop.
              // - Another thread just joined the (already finished) computation. The loop will continue.
              value = new;
            }
          }
        }
      }
    }
    self.state = EmptySignalState::DidSignal;
  }
}
