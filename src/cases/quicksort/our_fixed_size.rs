// A fully parallel implementation of quicksort.
// - Not inplace, one additional array is used
// - Parallel partition with data parallelism
// - Two sections are sorted in parallel with task parallelism

use core::sync::atomic::{Ordering, AtomicU32, AtomicU64};
use crate::cases::quicksort::parallel_partition_block_specialized;
use crate::core::worker::*;
use crate::core::workstealing_loop::*;
use crate::core::task::*;

use crate::cases::quicksort::{SEQUENTIAL_CUTOFF, DATAPAR_CUTOFF, BLOCK_SIZE};
use crate::cases::quicksort::sequential;
use crate::cases::quicksort::task_parallel;

struct Data<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a [AtomicU32],
  output: &'a [AtomicU32],
  input_output_flipped: bool,
  // 32 least significant bits are used for the number of items on the left side,
  // 32 most significat bits are used for the number of items on the right side
  counters: AtomicU64
}

pub fn create_task<'a>(pending_tasks: &'a AtomicU64, input: &'a [AtomicU32], output: &'a [AtomicU32], input_output_flipped: bool) -> Option<Task> {
  assert_eq!(input.len(), output.len());

  if input.len() == 0 {
    return None
  } else if input.len() == 1 {
    if !input_output_flipped {
      output[0].store(input[0].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    return None;
  }

  pending_tasks.fetch_add(1, Ordering::Relaxed);

  if input.len() < SEQUENTIAL_CUTOFF {
    return Some(sequential::create_task(pending_tasks, input, if input_output_flipped { None } else { Some(output) }));
  }

  if input.len() < DATAPAR_CUTOFF {
    if input_output_flipped {
      let data = task_parallel::Sort{
        pending_tasks,
        array: input
      };
      return Some(Task::new_single(task_parallel::run, data));
    } else {
      let data = task_parallel::SortWithCopy{
        pending_tasks,
        input,
        output
      };
      return Some(Task::new_single(task_parallel::run_with_copy, data));
    }
  }

  let data = Data{
    pending_tasks,
    input,
    output,
    input_output_flipped,
    counters: AtomicU64::new(0)
  };

  Some(Task::new_dataparallel(partition_run, partition_finish, data, ((input.len() - 1 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32))
}

fn partition_run(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let pivot = data.input[0].load(Ordering::Relaxed);

  workstealing_loop!(loop_arguments, |block_index| {
    parallel_partition_block_specialized(data.input, data.output, pivot, &data.counters, block_index as usize);
  });
}

fn partition_finish(workers: &Workers, data: &Data) {
  let counters = data.counters.load(Ordering::Relaxed);
  let count_left = counters & 0xFFFFFFFF;
  let count_right = counters >> 32;
  assert_eq!(count_left + count_right + 1, data.input.len() as u64);

  let pivot = data.input[0].load(Ordering::Relaxed);
  (if data.input_output_flipped { data.input } else { data.output })
    [count_left as usize].store(pivot, Ordering::Relaxed);

  for (from, to) in [(0, count_left as usize), (count_left as usize + 1, data.input.len())] {
    if let Some(task) = create_task(data.pending_tasks, &data.output[from .. to], &data.input[from .. to], !data.input_output_flipped) {
      workers.push_task(task);
    }
  }

  if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
    workers.finish();
  }
}
