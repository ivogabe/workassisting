use core::sync::atomic::{Ordering, AtomicU64};
use rayon::prelude::*;
use crate::core::worker::*;
use crate::utils::benchmark::{benchmark, ChartStyle, Nesting};
use crate::utils::thread_pinning::AFFINITY_MAPPING;
use num_format::{Locale, ToFormattedString};

mod deque;
mod our;
mod our_fixed_size;

pub const BLOCK_SIZE: u64 = 2048 * 2;

pub const START: u64 = 1024 * 1024 * 1024;

pub fn run(open_mp_enabled: bool) {
  for count in [1024 * 1024 * 32 + 1234, 1024 * 1024 * 128 + 1234, 1024 * 1024 * 1024 + 1234] {
    let name = "Sum function (n = ".to_owned() + &(count).to_formatted_string(&Locale::en) + ")";
    benchmark(
      ChartStyle::Right,
      &name,
      || reference_sequential_single(count)
    )
      .rayon(None, || reference_parallel(count))
      .naive_parallel(|thread_count, pinned| naive(count, thread_count, pinned))
      .work_stealing(|thread_count| {
        deque::sum(START, count, thread_count)
      })
      .open_mp(open_mp_enabled, "OpenMP (static)", 6, "sum-function-static", Nesting::Flat, count as usize, None)
      .open_mp(open_mp_enabled, "OpenMP (dynamic)", 7, "sum-function-dynamic", Nesting::Flat, count as usize, None)
      .our(|thread_count| {
        let counter = AtomicU64::new(0);
        let task = our::create_task(&counter, START, count);
        Workers::run(thread_count, task);
        counter.load(Ordering::Acquire)
      })
      .our_fixed_size(|thread_count| {
        let counter = AtomicU64::new(0);
        let task = our_fixed_size::create_task(&counter, START, count);
        Workers::run(thread_count, task);
        counter.load(Ordering::Acquire)
      });
  }
}

#[inline(always)]
pub fn random(mut seed: u64) -> u32 {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  seed = seed.wrapping_mul(seed);
  seed += 9023;
  seed = (seed as f64).sqrt() as u64;
  seed ^= seed >> 11;
  seed ^= seed << 9;
  (seed & 0xFFFFFFF) as u32
}

pub fn reference_sequential_single(count: u64) -> u64 {
  let mut counter = 0;
  for number in START .. START + count {
    counter += random(number) as u64;
  }
  counter
}

pub fn reference_parallel(count: u64) -> u64 {
  (START .. START + count).into_par_iter().map(|x| random(x) as u64).sum()
}

pub fn naive(count: u64, thread_count: usize, pinned: bool) -> u64 {
  let result = AtomicU64::new(0);
  crossbeam::scope(|s| {
    let result_ref = &result;
    for thread_index in 0 .. thread_count {
      s.spawn(move |_| {
        if pinned {
          affinity::set_thread_affinity([AFFINITY_MAPPING[thread_index]]).unwrap();
        }
        let start = START + thread_index as u64 * count / thread_count as u64;
        let end = START + (thread_index as u64 + 1) * count / thread_count as u64;
        let mut sum = 0;
        for idx in start .. end {
          sum += random(idx) as u64;
        }
        result_ref.fetch_add(sum, Ordering::Relaxed);
      });
    }
  }).unwrap();
  result.load(Ordering::Relaxed)
}

