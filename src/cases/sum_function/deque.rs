use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::cases::sum_function;
use crate::utils::deque_stealer::*;

pub fn sum(first: u64, length: u64, thread_count: usize) -> u64 {
  let pending_tasks = AtomicU32::new(1);
  let counter = AtomicU64::new(0);
  let task = Task::new(run, Box::new(Data{ pending: &pending_tasks, counter: &counter, first, length }));
  run_with_workstealing(vec![task], thread_count);
  counter.load(Ordering::Relaxed)
}

struct Data<'a> {
  pending: &'a AtomicU32,
  counter: &'a AtomicU64,
  first: u64,
  length: u64
}

fn run(worker: Worker, data_box: Box<Data>) {
  let data = *data_box;
  let end = data.first + data.length;
  let mut own_end = end;

  if data.length >= 2 * sum_function::BLOCK_SIZE {
    let own = sum_function::BLOCK_SIZE;
    let other = data.length - own;
    own_end = data.first + own;

    if other < 2 * sum_function::BLOCK_SIZE {
      data.pending.fetch_add(1, Ordering::Relaxed);
      let subtask1 = Data{ pending: data.pending, counter: data.counter, first: own_end, length: other };
      worker.push_task(Task::new(run, Box::new(subtask1)));
    } else {
      data.pending.fetch_add(2, Ordering::Relaxed);
      let mid = other / 2;
      let subtask1 = Data{ pending: data.pending, counter: data.counter, first: own_end, length: mid };
      worker.push_task(Task::new(run, Box::new(subtask1)));
      let subtask2 = Data{ pending: data.pending, counter: data.counter, first: own_end + mid, length: other - mid };
      worker.push_task(Task::new(run, Box::new(subtask2)));
    }
  }
  let mut local_count = 0;
  for i in data.first .. own_end {
    local_count += sum_function::random(i) as u64;
  }
  data.counter.fetch_add(local_count, Ordering::Relaxed);
  if data.pending.fetch_sub(1, Ordering::Relaxed) == 1 {
    worker.finish();
  }
}
