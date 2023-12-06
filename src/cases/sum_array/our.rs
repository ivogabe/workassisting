use core::sync::atomic::{Ordering, AtomicU64};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;
use crate::utils::loops::*;
use crate::cases::sum_array;

struct Data<'a> {
  counter: &'a AtomicU64,
  array: &'a [u64]
}

pub fn create_task(counter: &AtomicU64, array: &[u64]) -> Task {
  Task::new_dataparallel::<Data>(work, finish, Data{ counter, array }, ((array.len() + sum_array::BLOCK_SIZE - 1) / sum_array::BLOCK_SIZE) as u32)
}

fn work(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };

  let mut local_count = 0;

  let counter = data.counter;
  workassisting_loop!(loop_arguments, |chunk_index| {
    let from = chunk_index as usize * sum_array::BLOCK_SIZE;
    let to = from + sum_array::BLOCK_SIZE;

    let mut local_local_count = 0;
    loop_fixed_size!(number in from, to, data.array.len(), {
      local_local_count += data.array[number];
    });
    local_count += local_local_count;
  });
  counter.fetch_add(local_count, Ordering::Relaxed);
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  unsafe {
    drop(Box::from_raw(task));
  }
  workers.finish();
}
