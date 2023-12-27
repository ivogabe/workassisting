use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

use crossbeam::deque;

use crate::utils::thread_pinning::AFFINITY_MAPPING;

pub struct Worker<'a> {
  worker: &'a deque::Worker<Task>,
  finished: &'a AtomicBool
}

impl<'a> Worker<'a> {
  pub fn finish(&self) {
    self.finished.store(true, Ordering::Relaxed);
  }

  pub fn push_task(&self, task: Task) {
    self.worker.push(task);
  }
}

pub struct Task {
  pub function: fn(worker: Worker, data: Box<()>) -> (),
  pub data: Box<()>,
}

impl Task {
  pub fn new<T>(function: fn(worker: Worker, data: Box<T>) -> (), data: Box<T>) -> Task {
    unsafe {
      Task{
        function: mem::transmute(function),
        data: mem::transmute(data)
      }
    }
  }
}

pub fn run_with_workstealing(initial_tasks: Vec<Task>, thread_count: usize) {
  let workers: Vec<deque::Worker<Task>> = (0 .. thread_count).into_iter().map(|_| deque::Worker::new_lifo()).collect();
  let stealers: Box<[deque::Stealer<Task>]> = workers.iter().map(|w| w.stealer()).collect();

  for (index, task) in initial_tasks.into_iter().enumerate() {
    workers[index % thread_count].push(task);
  }

  let finished = AtomicBool::new(false);

  let full = affinity::get_thread_affinity().unwrap();
  std::thread::scope(|s| {
    for (thread_index, worker) in workers.into_iter().enumerate() {
      affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
      let stealers_ref = &stealers;
      let finished_ref = &finished;
      s.spawn(move || {
        while let Some(item) = claim(thread_index, thread_count, &worker, stealers_ref, finished_ref) {
          execute_task(Worker{ worker: &worker, finished: finished_ref }, item);
        }
      });
    }
    affinity::set_thread_affinity(full).unwrap();
  });
}

fn claim<T>(thread_index: usize, thread_count: usize, local: &deque::Worker<T>, stealers: &[deque::Stealer<T>], finished: &AtomicBool) -> Option<T> {
  if finished.load(Ordering::Relaxed) {
    return None;
  }
  if let Some(item) = local.pop() {
    return Some(item);
  }
  loop {
    let mut other_index = thread_index;
    let increment = if thread_index % 2 == 0 { 1 } else { thread_count - 1 };
    loop {
      other_index = (other_index + increment) % thread_count;
      if other_index == thread_index {
        break;
      }
      if let Some(item) = stealers[other_index].steal().success() {
        return Some(item);
      }
    }
    if finished.load(Ordering::Relaxed) {
      return None;
    }
  }
}

fn execute_task(worker: Worker, task: Task) {
  (task.function)(worker, task.data);
}
