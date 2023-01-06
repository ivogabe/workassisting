use core::sync::atomic::{Ordering, AtomicU32};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workstealing_loop::*;
use crate::cases::prime;

struct Data<'a> {
  counter: &'a AtomicU32,
  first: u64,
  length: u64
}

pub fn create_task(counter: &AtomicU32, first: u64, length: u64) -> Task {
  Task::new_dataparallel::<Data>(go, finish, Data{ counter, first, length }, ((length + prime::BLOCK_SIZE - 1) / prime::BLOCK_SIZE) as u32)
}

fn go(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let mut local_count = 0;

  workstealing_loop!(loop_arguments, |block_index| {
    let mut local_local_count = 0;
    let end = (data.first + (block_index as u64 + 1) * prime::BLOCK_SIZE).min(data.first + data.length);
    for number in data.first + block_index as u64 * prime::BLOCK_SIZE .. end {
      if prime::is_prime(number) {
        local_local_count += 1;
      }
    }
    local_count += local_local_count;
  });
  data.counter.fetch_add(local_count, Ordering::Relaxed);
}

fn finish(workers: &Workers, _data: &Data) {
  workers.finish();
}
