// A fully parallel implementation of quicksort.
// - Not inplace, one additional array is used
// - Parallel partition with task parallelism
// - Two sections are sorted in parallel with task parallelism

use core::sync::atomic::{Ordering, AtomicU32, AtomicU64};

use crate::cases::quicksort::{random, SEQUENTIAL_CUTOFF, DATAPAR_CUTOFF, BLOCK_SIZE, parallel_partition_chunk};
use crate::cases::quicksort::sequential;

use crate::utils::deque_stealer::*;

use super::count_recursive_calls;

pub fn reset_and_sort(input: &[AtomicU32], output: &[AtomicU32], thread_count: usize) {
  let pending_tasks = AtomicU64::new(thread_count as u64);
  let tasks = (0..thread_count).map(|idx|
    Task::new(run_reset, Box::new(TaskReset { pending_tasks: &pending_tasks, input, output, start: (input.len() * idx) / thread_count, end: (input.len() * (idx + 1)) / thread_count }))
  ).collect();
  run_with_workstealing(tasks, thread_count);
}

struct Sort<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a [AtomicU32],
  output: &'a [AtomicU32],
  input_output_flipped: bool,
  reference_count: AtomicU64,
  // 32 least significant bits are used for the number of items on the left side,
  // 32 most significat bits are used for the number of items on the right side
  counters: AtomicU64
}

struct TaskReset<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a [AtomicU32],
  output: &'a [AtomicU32],
  start: usize,
  end: usize
}
struct TaskParallelPartition<'a> {
  sort: *mut Sort<'a>,
  start_block: usize,
  end_block: usize
}
unsafe impl<'a> Send for TaskParallelPartition<'a> {}
unsafe impl<'a> Sync for TaskParallelPartition<'a> {}
struct TaskRecursionParallel<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a[AtomicU32],
  // The output array, if different from the input array. When this is None, the input array is also used for the output (inplace).
  output: Option<&'a [AtomicU32]>,
}
struct TaskSequential<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a[AtomicU32],
  // The output array, if different from the input array. When this is None, the input array is also used for the output (inplace).
  output: Option<&'a [AtomicU32]>,
}

fn run_reset(worker: Worker, data_box: Box<TaskReset>) {
  let data = *data_box;
  for index in data.start .. data.end {
    data.input[index].store(random(index as u64), Ordering::Relaxed);
  }
  if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
    // Reset is finished. Now start the actual work
    if let Some(task) = create_task(&data.pending_tasks, data.input, data.output, false) {
      worker.push_task(task);
    } else {
      worker.finish();
    }
  }
}

fn run_sequential(worker: Worker, data_box: Box<TaskSequential>) {
  let data = *data_box;
  let array = if let Some(o) = data.output {
    for i in 0 .. o.len() {
      o[i].store(data.input[i].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    o
  } else {
    data.input
  };
  sequential::sort(array);
  if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
    // The entire sort is sorted
    worker.finish();
  }
}

fn run_recursion_parallel(worker: Worker, data_box: Box<TaskRecursionParallel>) {
  let data = *data_box;
  let array = if let Some(o) = data.output {
    for i in 0 .. o.len() {
      o[i].store(data.input[i].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    o
  } else {
    data.input
  };
  // Sequential partition, recursive calls in parallel
  let pivot = array[0].load(Ordering::Relaxed);
  let pivot_idx = sequential::partition(array);

  // Pivot should be placed at index 'pivot_idx'.
  array[0].store(array[pivot_idx].load(Ordering::Relaxed), Ordering::Relaxed);
  array[pivot_idx].store(pivot, Ordering::Relaxed);

  let n = count_recursive_calls(array.len(), pivot_idx as usize);
  match n {
    2 => {
      data.pending_tasks.fetch_add(1, Ordering::Relaxed);
    },
    0 => {
      if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
        worker.finish();
      }
    },
    _ => {} // No work to be done if there is one recursive call,
    // As the number of pending tasks doesn't change.
  }

  let mut m = 0;
  if let Some(t) = create_task_task_parallel(data.pending_tasks, &array[0 .. pivot_idx]) {
    worker.push_task(t);
    m += 1;
  }
  if let Some(t) = create_task_task_parallel(data.pending_tasks, &array[pivot_idx + 1 ..]) {
    worker.push_task(t);
    m += 1;
  }
  if n != m {
    println!("{}!={}, {} {}", n, m, array.len(), pivot_idx);
  }
}

fn run_parallel_partition(worker: Worker, data_box: Box<TaskParallelPartition>) {
  let data = *data_box;
  let mut own_end = data.end_block;
  let len = data.end_block - data.start_block;
  let task = unsafe { &*data.sort };

  if len >= 2 {
    let own = 1;
    let other = len - own;
    own_end = data.start_block + own;

    if other < 2 {
      task.reference_count.fetch_add(1, Ordering::Relaxed);
      let subtask1 = TaskParallelPartition{ sort: data.sort, start_block: own_end, end_block: data.end_block };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask1)));
    } else {
      task.reference_count.fetch_add(2, Ordering::Relaxed);
      // Choose the length of subtask1 as a multiple of the chunk size
      let mid = own_end + other / 2 / BLOCK_SIZE * BLOCK_SIZE;
      let subtask1 = TaskParallelPartition{ sort: data.sort, start_block: own_end, end_block: mid };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask1)));
      let subtask2 = TaskParallelPartition{ sort: data.sort, start_block: mid, end_block: data.end_block };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask2)));
    }
  }

  let pivot = task.input[0].load(Ordering::Relaxed);
  for idx in data.start_block .. own_end {
    parallel_partition_chunk(task.input, task.output, pivot, &task.counters, idx);
  }

  if task.reference_count.fetch_sub(1, Ordering::AcqRel) == 1 {
    // This task is finished
    let counters = task.counters.load(Ordering::Relaxed);
    let count_left = counters & 0xFFFFFFFF;
    let count_right = counters >> 32;
    assert_eq!(count_left + count_right + 1, task.input.len() as u64);

    let pivot = task.input[0].load(Ordering::Relaxed);
    (if task.input_output_flipped { task.input } else { task.output })
      [count_left as usize].store(pivot, Ordering::Relaxed);

    for (from, to) in [(0, count_left as usize), (count_left as usize + 1, task.input.len())] {
      if let Some(task) = create_task(task.pending_tasks, &task.output[from .. to], &task.input[from .. to], !task.input_output_flipped) {
        worker.push_task(task);
      }
    }

    if task.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
      // The entire sort is sorted
      worker.finish();
    }

    // Take ownership of task object to deallocate it.
    let _sort_owned = unsafe { Box::from_raw(data.sort) };
  }
}

fn create_task<'a>(pending_tasks: &'a AtomicU64, input: &'a [AtomicU32], output: &'a [AtomicU32], input_output_flipped: bool) -> Option<Task> {
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
    if input_output_flipped {
      return Some(Task::new(run_sequential, Box::new(TaskSequential{ pending_tasks, input, output: None })));
    } else {
      return Some(Task::new(run_sequential, Box::new(TaskSequential{ pending_tasks, input, output: Some(output) })));
    }
  }

  if input.len() < DATAPAR_CUTOFF {
    if input_output_flipped {
      return Some(Task::new(run_recursion_parallel, Box::new(TaskRecursionParallel{ pending_tasks, input, output: None })));
    } else {
      return Some(Task::new(run_recursion_parallel, Box::new(TaskRecursionParallel{ pending_tasks, input, output: Some(output) })));
    }
  }

  let data = Box::new(Sort{
    pending_tasks,
    input,
    output,
    input_output_flipped,
    reference_count: AtomicU64::new(1),
    counters: AtomicU64::new(0)
  });
  let blocks = (input.len() - 1 + BLOCK_SIZE - 1) / BLOCK_SIZE;
  let subtask = TaskParallelPartition{ sort: Box::into_raw(data), start_block: 0, end_block: blocks };

  Some(Task::new(run_parallel_partition, Box::new(subtask)))
}

fn create_task_task_parallel<'a>(pending_tasks: &'a AtomicU64, array: &'a [AtomicU32]) -> Option<Task> {
  if array.len() <= 1 {
    return None
  }

  if array.len() < SEQUENTIAL_CUTOFF {
    return Some(Task::new(run_sequential, Box::new(TaskSequential{ pending_tasks, input: array, output: None })));
  }
  Some(Task::new(run_recursion_parallel, Box::new(TaskRecursionParallel{ pending_tasks, input: array, output: None })))
}
