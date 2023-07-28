use core::sync::atomic::{Ordering, AtomicU32, AtomicU64};
use num_format::{Locale, ToFormattedString};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;
use crate::specialize_if;
use crate::utils::array::alloc_undef_u32_array;
use crate::utils::benchmark::ChartStyle;
use crate::utils::benchmark::benchmark;

pub mod our;
pub mod our_fixed_size;
pub mod deque_parallel_partition;
pub mod sequential;
pub mod task_parallel;

pub const BLOCK_SIZE: usize = 1024;

pub const DATAPAR_CUTOFF: usize = 1024 * 32;
pub const SEQUENTIAL_CUTOFF: usize = 1024 * 8;

pub fn run(open_mp_enabled: bool) {
  run_on(open_mp_enabled, 1024 * 1024);
  run_on(open_mp_enabled, 1024 * 1024 * 16);
  run_on(open_mp_enabled, 1024 * 1024 * 64);
}

fn run_on(open_mp_enabled: bool, size: usize) {
  let array1 = unsafe { alloc_undef_u32_array(size) };
  let array2 = unsafe { alloc_undef_u32_array(size) };
  let name = "Sort (n = ".to_owned() + &size.to_formatted_string(&Locale::en) + ")";
  benchmark(
    if size == 1024 * 1024 * 64 { ChartStyle::Right } else { ChartStyle::LeftWithKey },
    &name,
    || reference_sequential_single(&array1)
  )
  .parallel("Sequential partition", 9, false, |thread_count| {
    let pending_tasks = AtomicU64::new(0);
    Workers::run(thread_count, create_task_reset(&array1, &pending_tasks, Kind::OnlyTaskParallel));
    assert_eq!(pending_tasks.load(Ordering::Relaxed), 0);
    output(&array1)
  })
  .work_stealing(|thread_count| {
    deque_parallel_partition::reset_and_sort(&array1, &array2, thread_count);
    output(&array2)
  })
  .open_mp(open_mp_enabled, "OpenMP (no nested parallelism)", 6, "quicksort", false, size, None)
  .open_mp(open_mp_enabled, "OpenMP (nested)", 7, "quicksort", true, size, None)
  .our(|thread_count| {
    let pending_tasks = AtomicU64::new(0);
    Workers::run(thread_count, create_task_reset(&array1, &pending_tasks, Kind::DataParallel(&array2, false)));
    output(&array2)
  })
  .our_fixed_size(|thread_count| {
    let pending_tasks = AtomicU64::new(0);
    Workers::run(thread_count, create_task_reset(&array1, &pending_tasks, Kind::DataParallel(&array2, true)));
    output(&array2)
  });
}

pub fn random(mut seed: u64) -> u32 {
  seed += 876998787696;
  seed = seed.wrapping_mul(35334534876231);
  seed ^= seed << 19;
  seed ^= seed >> 23;
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  (seed & 0xFFFFFFFF) as u32
}

fn create_task_reset(array: &[AtomicU32], pending_tasks: &AtomicU64, kind: Kind) -> Task {
  let data = Reset{ array, pending_tasks, kind };
  Task::new_dataparallel(reset_run, reset_finish, data, ((array.len() + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32)
}

struct Reset<'a> {
  array: &'a [AtomicU32],

  // Info for next task
  pending_tasks: &'a AtomicU64,
  kind: Kind<'a>
}
enum Kind<'a> {
  OnlyTaskParallel,
  DataParallel(&'a [AtomicU32], bool)
}

fn reset_run(_workers: &Workers, data: &Reset, loop_arguments: LoopArguments) {
  workassisting_loop!(loop_arguments, |block_index| {
    for index in block_index as usize * BLOCK_SIZE .. ((block_index as usize + 1) * BLOCK_SIZE).min(data.array.len()) {
      data.array[index as usize].store(random(index as u64), Ordering::Relaxed);
    }
  });
}

fn reset_finish(workers: &Workers, data: &Reset) {
  match data.kind {
    Kind::OnlyTaskParallel => {
      workers.push_task(task_parallel::create_task(data.pending_tasks, data.array).unwrap());
    },
    Kind::DataParallel(output, false) => {
      workers.push_task(our::create_task(data.pending_tasks, data.array, output, false).unwrap());
    },
    Kind::DataParallel(output, true) => {
      workers.push_task(our_fixed_size::create_task(data.pending_tasks, data.array, output, false).unwrap());
    }
  }
}

fn output(array: &[AtomicU32]) -> u64 {
  array[0].load(Ordering::Relaxed) as u64
    + array[478].load(Ordering::Relaxed) as u64
    + array[array.len() / 2].load(Ordering::Relaxed) as u64
    + array[array.len() - 324].load(Ordering::Relaxed) as u64
    + array[array.len() - 1].load(Ordering::Relaxed) as u64
}

fn reference_sequential_single(array: &[AtomicU32]) -> u64 {
  for i in 0 .. array.len() {
    array[i].store(random(i as u64), Ordering::Relaxed);
  }
  sequential::sort(array);
  output(array)
}

#[inline(always)]
pub fn parallel_partition_block(input: &[AtomicU32], output: &[AtomicU32], pivot: u32, counters: &AtomicU64, block_index: usize) {
  // Loop starts at 1, as element 0 is the pivot.
  let start = 1 + block_index as usize * BLOCK_SIZE ;
  let end = 1 + ((block_index as usize + 1) * BLOCK_SIZE).min(input.len() - 1);

  let mut values = [0; BLOCK_SIZE];
  let mut left_count = 0;
  let mut right_count = 0;
  for i in 0 .. end - start {
    values[i] = input[start + i].load(Ordering::Relaxed);
    if values[i] < pivot {
      left_count += 1;
    } else {
      right_count += 1;
    }
  }
  let counters_value = counters.fetch_add((right_count << 32) | left_count, Ordering::Relaxed);
  let mut left_offset = (counters_value & 0xFFFFFFFF) as usize;
  let mut right_offset = input.len() - 1 - (counters_value >> 32) as usize;
  for i in 0 .. end - start {
    let destination;
    if values[i] < pivot {
      destination = left_offset;
      left_offset += 1;
    } else {
      destination = right_offset;
      right_offset -= 1;
    }
    output[destination].store(values[i], Ordering::Relaxed);
  }
}

#[inline(always)]
pub fn parallel_partition_block_specialized(input: &[AtomicU32], output: &[AtomicU32], pivot: u32, counters: &AtomicU64, block_index: usize) {
  // Loop starts at 1, as element 0 is the pivot.
  let start = 1 + block_index as usize * BLOCK_SIZE ;

  specialize_if!(start + BLOCK_SIZE <= input.len(), start + BLOCK_SIZE, input.len(), |end| {
    let mut values = [0; BLOCK_SIZE];
    let mut left_count = 0;
    let mut right_count = 0;
    for i in 0 .. end - start {
      values[i] = input[start + i].load(Ordering::Relaxed);
      if values[i] < pivot {
        left_count += 1;
      } else {
        right_count += 1;
      }
    }
    let counters_value = counters.fetch_add((right_count << 32) | left_count, Ordering::Relaxed);
    let mut left_offset = (counters_value & 0xFFFFFFFF) as usize;
    let mut right_offset = input.len() - 1 - (counters_value >> 32) as usize;
    for i in 0 .. end - start {
      let destination;
      if values[i] < pivot {
        destination = left_offset;
        left_offset += 1;
      } else {
        destination = right_offset;
        right_offset -= 1;
      }
      output[destination].store(values[i], Ordering::Relaxed);
    }
  });
}
