use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::cases::sum_array;
use crate::utils::deque_stealer::*;

pub fn sum(array: &[u64], thread_count: usize) -> u64 {
  let pending_tasks = AtomicU32::new(1);
  let counter = AtomicU64::new(0);
  let task = Task::new(run, Box::new(Data{ pending: &pending_tasks, counter: &counter, array }));
  run_with_workstealing(vec![task], thread_count);
  counter.load(Ordering::Relaxed)
}

struct Data<'a> {
  pending: &'a AtomicU32,
  counter: &'a AtomicU64,
  array: &'a [u64]
}

fn run(worker: Worker, data_box: Box<Data>) {
  let data = *data_box;
  let mut own_length = data.array.len();

  if own_length >= 2 * sum_array::BLOCK_SIZE as usize {
    own_length = sum_array::BLOCK_SIZE;
    let other = data.array.len() - own_length;

    if other < 2 * sum_array::BLOCK_SIZE {
      data.pending.fetch_add(1, Ordering::Relaxed);
      let subtask1 = Data{ pending: data.pending, counter: data.counter, array: &data.array[own_length ..] };
      worker.push_task(Task::new(run, Box::new(subtask1)));
    } else {
      data.pending.fetch_add(2, Ordering::Relaxed);
      let mid = own_length + other / 2;
      let subtask1 = Data{ pending: data.pending, counter: data.counter, array: &data.array[own_length .. mid] };
      worker.push_task(Task::new(run, Box::new(subtask1)));
      let subtask2 = Data{ pending: data.pending, counter: data.counter, array: &data.array[mid ..] };
      worker.push_task(Task::new(run, Box::new(subtask2)));
    }
  }
  let mut local_count = 0;
  for i in 0 .. own_length {
    local_count += data.array[i];
  }
  data.counter.fetch_add(local_count, Ordering::Relaxed);
  if data.pending.fetch_sub(1, Ordering::Relaxed) == 1 {
    worker.finish();
  }
}
