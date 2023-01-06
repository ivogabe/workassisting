use core::sync::atomic::{Ordering, AtomicU64};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workstealing_loop::*;
use crate::cases::sum_function;

struct Data<'a> {
  counter: &'a AtomicU64,
  first: u64,
  length: u64
}

pub fn create_task(counter: &AtomicU64, first: u64, length: u64) -> Task {
  Task::new_dataparallel::<Data>(work, finish, Data{ counter, first, length }, ((length + sum_function::BLOCK_SIZE - 1) / sum_function::BLOCK_SIZE) as u32)
}

fn work(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let mut local_count = 0;
  
  let first = data.first;
  let length = data.length;
  let counter = data.counter;
  workstealing_loop!(loop_arguments, |block_index| {
    let from = first + block_index as u64 * sum_function::BLOCK_SIZE;
    let to = from + sum_function::BLOCK_SIZE;

    let mut local_local_count = 0;
    for number in from .. to.min(first + length) {
      local_local_count += sum_function::random(number) as u64;
    };
    local_count += local_local_count;
  });
  counter.fetch_add(local_count, Ordering::Relaxed);
}

fn finish(workers: &Workers, _data: &Data) {
  workers.finish();
}