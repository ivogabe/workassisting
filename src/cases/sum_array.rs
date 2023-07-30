use core::sync::atomic::{Ordering, AtomicU64};
use rayon::prelude::*;
use crate::core::worker::*;
use crate::utils::benchmark::{benchmark, ChartStyle, Nesting};
use crate::utils::thread_pinning::AFFINITY_MAPPING;
use num_format::{Locale, ToFormattedString};

mod deque;
mod our;
mod our_fixed_size;

pub const BLOCK_SIZE: usize = 2048 * 4;

pub const START: u64 = 1024 * 1024 * 1024;

pub fn run(open_mp_enabled: bool) {
  for count in [1024 * 1024 * 32 + 1234, 1024 * 1024 * 128 + 1234, 1024 * 1024 * 1024 + 1234] {
    let name = "Sum array (n = ".to_owned() + &(count).to_formatted_string(&Locale::en) + ")";
    let array: Vec<u64> = (START .. START + count).map(|number| crate::cases::sum_function::random(number) as u64).collect();

    benchmark(
      ChartStyle::LeftWithKey,
      &name,
      || reference_sequential_single(&array)
    )
      .rayon(None, || reference_parallel(&array))
      .static_parallel(|thread_count, pinned| static_parallel(&array, thread_count, pinned))
      .work_stealing(|thread_count| {
        deque::sum(&array, thread_count)
      })
      .open_mp(open_mp_enabled, "OpenMP (static)", 6, "sum-array-static", Nesting::Flat, count as usize, None)
      .open_mp(open_mp_enabled, "OpenMP (dynamic)", 7, "sum-array-dynamic", Nesting::Flat, count as usize, None)
      .our(|thread_count| {
        let counter = AtomicU64::new(0);
        let task = our::create_task(&counter, &array);
        Workers::run(thread_count, task);
        counter.load(Ordering::Acquire)
      })
      .our_fixed_size(|thread_count| {
        let counter = AtomicU64::new(0);
        let task = our_fixed_size::create_task(&counter, &array);
        Workers::run(thread_count, task);
        counter.load(Ordering::Acquire)
      });
  }
}

pub fn reference_sequential_single(array: &[u64]) -> u64 {
  let mut counter = 0;
  for number in array {
    counter += *number;
  }
  counter
}

pub fn reference_parallel(array: &[u64]) -> u64 {
  array.into_par_iter().sum()
}

pub fn static_parallel(array: &[u64], thread_count: usize, pinned: bool) -> u64 {
  let result = AtomicU64::new(0);
  crossbeam::scope(|s| {
    let result_ref = &result;
    for thread_index in 0 .. thread_count {
      s.spawn(move |_| {
        if pinned {
          affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
        }
        let start = thread_index * array.len() / thread_count;
        let end = (thread_index + 1) * array.len() / thread_count;
        let mut sum = 0;
        for idx in start .. end {
          sum += array[idx];
        }
        result_ref.fetch_add(sum, Ordering::Relaxed);
      });
    }
  }).unwrap();
  result.load(Ordering::Relaxed)
}

