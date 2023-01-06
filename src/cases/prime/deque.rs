use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use crate::cases::prime;
use crate::utils::deque_stealer::*;

pub fn count_primes(start: u64, length: u64, thread_count: usize) -> u32 {
  let pending_tasks = AtomicU32::new(1);
  let counter = AtomicU32::new(0);
  let task = Task::new(run, Box::new(Data{ pending: &pending_tasks, counter: &counter, start, end: start + length }));
  run_with_workstealing(vec![task], thread_count);
  counter.load(Ordering::Relaxed)
}

struct Data<'a> {
  pending: &'a AtomicU32,
  counter: &'a AtomicU32,
  start: u64,
  end: u64
}

fn run(worker: Worker, data_box: Box<Data>) {
  let data = *data_box;
  let mut own_end = data.end;
  let len = data.end - data.start;

  if len >= 2 * prime::BLOCK_SIZE {
    let own = prime::BLOCK_SIZE;
    let other = len - own;
    own_end = data.start + own;

    if other < 2 * prime::BLOCK_SIZE {
      data.pending.fetch_add(1, Ordering::Relaxed);
      let subtask1 = Data{ pending: data.pending, counter: data.counter, start: own_end, end: data.end };
      worker.push_task(Task::new(run, Box::new(subtask1)));
    } else {
      data.pending.fetch_add(2, Ordering::Relaxed);
      let mid = own_end + other / 2;
      let subtask1 = Data{ pending: data.pending, counter: data.counter, start: own_end, end: mid };
      worker.push_task(Task::new(run, Box::new(subtask1)));
      let subtask2 = Data{ pending: data.pending, counter: data.counter, start: mid, end: data.end };
      worker.push_task(Task::new(run, Box::new(subtask2)));
    }
  }
  let mut local_count = 0;
  for i in data.start .. own_end {
    if prime::is_prime(i) {
      local_count += 1;
    }
  }
  data.counter.fetch_add(local_count, Ordering::Relaxed);
  if data.pending.fetch_sub(1, Ordering::Relaxed) == 1 {
    worker.finish();
  }
}
