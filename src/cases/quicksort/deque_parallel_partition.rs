// A fully parallel implementation of quicksort.
// - Not inplace, one additional array is used
// - Parallel partition with task parallelism
// - Two sections are sorted in parallel with task parallelism

use core::sync::atomic::{Ordering, AtomicU32, AtomicU64};

use crate::cases::quicksort::{random, SEQUENTIAL_CUTOFF, DATAPAR_CUTOFF, BLOCK_SIZE};
use crate::cases::quicksort::sequential;

use crate::utils::deque_stealer::*;

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
  start: usize,
  end: usize
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

  if let Some(t) = create_task_task_parallel(data.pending_tasks, &array[0 .. pivot_idx]) {
    worker.push_task(t);
  }
  if let Some(t) = create_task_task_parallel(data.pending_tasks, &array[pivot_idx + 1 ..]) {
    worker.push_task(t);
  }
  if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
    // The entire sort is sorted
    worker.finish();
  }
}

fn run_parallel_partition(worker: Worker, data_box: Box<TaskParallelPartition>) {
  let data = *data_box;
  let mut own_end = data.end;
  let len = data.end - data.start;
  let task = unsafe { &*data.sort };

  if len >= 4 * BLOCK_SIZE {
    let own = BLOCK_SIZE * 2;
    let other = len - own;
    own_end = data.start + own;

    if other < 2 * BLOCK_SIZE {
      task.reference_count.fetch_add(1, Ordering::Relaxed);
      let subtask1 = TaskParallelPartition{ sort: data.sort, start: own_end, end: data.end };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask1)));
    } else {
      task.reference_count.fetch_add(2, Ordering::Relaxed);
      // Choose the length of subtask1 as a multiple of the block size
      let mid = own_end + other / 2 / BLOCK_SIZE * BLOCK_SIZE;
      let subtask1 = TaskParallelPartition{ sort: data.sort, start: own_end, end: mid };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask1)));
      let subtask2 = TaskParallelPartition{ sort: data.sort, start: mid, end: data.end };
      worker.push_task(Task::new(run_parallel_partition, Box::new(subtask2)));
    }
  }

  let pivot = task.input[0].load(Ordering::Relaxed);
  let mut idx = data.start;

  const CHUNK_SIZE: usize = BLOCK_SIZE;
  while idx < own_end {
    let mut values = [0; CHUNK_SIZE];
    let mut left_count = 0;
    let mut right_count = 0;
    for i in 0 .. CHUNK_SIZE {
      if idx + i >= own_end { break; }
      values[i] = task.input[idx + i].load(Ordering::Relaxed);
      if values[i] < pivot {
        left_count += 1;
      } else {
        right_count += 1;
      }
    }
    let counters = task.counters.fetch_add((right_count << 32) | left_count, Ordering::Relaxed);
    let mut left_offset = (counters & 0xFFFFFFFF) as usize;
    let mut right_offset = task.input.len() - 1 - (counters >> 32) as usize;
    for i in 0 .. CHUNK_SIZE {
      if idx + i >= own_end { break; }
      let destination;
      if values[i] < pivot {
        destination = left_offset;
        left_offset += 1;
      } else {
        destination = right_offset;
        right_offset -= 1;
      }
      task.output[destination].store(values[i], Ordering::Relaxed);
    }
    idx += CHUNK_SIZE;
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
  let subtask = TaskParallelPartition{ sort: Box::into_raw(data), start: 1, end: input.len() };

  Some(Task::new(run_parallel_partition, Box::new(subtask)))
}

fn create_task_task_parallel<'a>(pending_tasks: &'a AtomicU64, array: &'a [AtomicU32]) -> Option<Task> {
  if array.len() <= 1 {
    return None
  }

  pending_tasks.fetch_add(1, Ordering::Relaxed);

  if array.len() < SEQUENTIAL_CUTOFF {
    return Some(Task::new(run_sequential, Box::new(TaskSequential{ pending_tasks, input: array, output: None })));
  }
  Some(Task::new(run_recursion_parallel, Box::new(TaskRecursionParallel{ pending_tasks, input: array, output: None })))
}
