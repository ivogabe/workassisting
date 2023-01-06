// A sequential implementation of quicksort.
// - Inplace
// - Sequential partition
// - Two sections are sorted sequentially with two recursive calls

use core::sync::atomic::{Ordering, AtomicU32, AtomicU64};

use crate::core::{worker::Workers, task::Task};

const INSERTION_SORT_CUTOFF: usize = 20;

pub fn sort<'a>(array: &'a [AtomicU32]) {
  if array.len() <= 1 {
    return;
  }
  if array.len() <= INSERTION_SORT_CUTOFF {
    insertion_sort(array);
    return;
  }

  let pivot = array[0].load(Ordering::Relaxed);
  let pivot_idx = partition(array);

  // Pivot should be placed at index 'pivot_idx'.
  array[0].store(array[pivot_idx].load(Ordering::Relaxed), Ordering::Relaxed);
  array[pivot_idx].store(pivot, Ordering::Relaxed);

  sort(&array[0 .. pivot_idx]);
  sort(&array[pivot_idx + 1 ..]);
}

pub fn partition(array: &[AtomicU32]) -> usize {
  assert!(array.len() > 1);

  let pivot = array[0].load(Ordering::Relaxed);
  let mut left = 1;
  let mut right = array.len() - 1;
  loop {
    while left < array.len() && array[left].load(Ordering::Relaxed) < pivot { left += 1; }
    while right > 0 && array[right].load(Ordering::Relaxed) >= pivot { right -= 1; }
    if left >= right { break; }
    let left_value = array[left].load(Ordering::Relaxed);
    array[left].store(array[right].load(Ordering::Relaxed), Ordering::Relaxed);
    array[right].store(left_value, Ordering::Relaxed);
    left += 1;
    right -= 1;
  }

  assert_eq!(left - 1, right);
  right
}

pub fn insertion_sort<'a>(array: &'a [AtomicU32]) {
  for idx in 1 .. array.len() {
    let value = array[idx].load(Ordering::Relaxed);
    // Find position for this element.
    let mut j = idx;
    while j > 0 {
      j -= 1;
      let current = array[j].load(Ordering::Relaxed);
      if current <= value {
        j += 1;
        break;
      }
      array[j + 1].store(current, Ordering::Relaxed);
    }
    array[j].store(value, Ordering::Relaxed);
  }
}

struct SequentialSort<'a> {
  pending_tasks: &'a AtomicU64,
  input: &'a [AtomicU32],
  output: Option<&'a [AtomicU32]>
}

fn sequential_sort_run(workers: &Workers, data: &SequentialSort) {
  let array = if let Some(output) = data.output {
    for i in 0 .. output.len() {
      output[i].store(data.input[i].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    output
  } else {
    data.input
  };

  sort(array);

  if data.pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
    workers.finish();
  }
}

pub fn create_task<'a>(pending_tasks: &'a AtomicU64, input: &'a [AtomicU32], output: Option<&'a [AtomicU32]>) -> Task {
  let data = SequentialSort{
    pending_tasks,
    input,
    output
  };
  Task::new_single(sequential_sort_run, data)
}
