use core::sync::atomic::{Ordering, AtomicU32};
use rayon::prelude::*;
use crate::core::worker::*;
use crate::utils::benchmark::ChartStyle;
use crate::utils::benchmark::benchmark;
use crate::utils::thread_pinning::AFFINITY_MAPPING;
use num_format::{Locale, ToFormattedString};

pub mod deque;
pub mod our;
pub mod our_fixed_size;

const BLOCK_SIZE: u64 = 32;

pub const START: u64 = 1024 * 1024 * 1024;
pub const COUNT: u64 = 1024 * 1024;

pub fn run() {
  run_on(ChartStyle::Left, 1, COUNT/2);
  run_on(ChartStyle::Left, 1, COUNT * 8);
  run_on(ChartStyle::Right, START, COUNT / 2);
}

fn run_on(style: ChartStyle, start: u64, count: u64) {
  let name = "Primes (".to_owned() + &start.to_formatted_string(&Locale::en) + " .. " + &(start + count).to_formatted_string(&Locale::en) + ")";
  benchmark(
    style,
    &name,
    || reference_sequential_single(start, count)
  )
  .rayon(None, || reference_parallel(start, count))
  .naive_parallel(|thread_count, pinned| naive(start, count, thread_count, pinned))
  .work_stealing(|thread_count| deque::count_primes(start, count, thread_count))
  .our(|thread_count| {
    let counter = AtomicU32::new(0);
    let task = our::create_task(&counter, start, count);
    Workers::run(thread_count, task);
    counter.load(Ordering::Acquire)
  })
  .our_fixed_size(|thread_count| {
    let counter = AtomicU32::new(0);
    let task = our_fixed_size::create_task(&counter, start, count);
    Workers::run(thread_count, task);
    counter.load(Ordering::Acquire)
  });
}

pub fn reference_sequential_single(start: u64, count: u64) -> u32 {
  let mut counter = 0;
  for number in start .. start + count {
    if is_prime(number) {
      counter += 1;
    }
  }
  counter
}

pub fn reference_parallel(start: u64, count: u64) -> u32 {
  (start .. start + count).into_par_iter().map(|x| if is_prime(x) { 1 } else { 0 }).sum()
}

pub fn naive(start: u64, count: u64, thread_count: usize, pinned: bool) -> u32 {
  let result = AtomicU32::new(0);
  crossbeam::scope(|s| {
    let result_ref = &result;
    for thread_index in 0 .. thread_count {
      s.spawn(move |_| {
        if pinned {
          affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
        }
        let local_start = start + thread_index as u64 * count / thread_count as u64;
        let local_end = start + (thread_index as u64 + 1) * count / thread_count as u64;
        let mut local_count = 0;
        for idx in local_start .. local_end {
          if is_prime(idx) {
            local_count += 1;
          }
        }
        result_ref.fetch_add(local_count, Ordering::Relaxed);
      });
    }
  }).unwrap();
  result.load(Ordering::Relaxed)
}

fn is_prime(input: u64) -> bool {
  // Check whether the input is even
  if input % 2 == 0 && input != 2 {
    return false;
  }

  // Check odd factors
  let mut factor = 3;
  while factor * factor <= input {
    if input % factor == 0 {
      return false;
    }
    factor += 2;
  }

  true
}
