use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use num_format::{Locale, ToFormattedString};

use crate::core::worker::Workers;
use crate::utils::benchmark::{benchmark_with_title, ChartLineStyle, ChartStyle, Nesting};
use crate::utils;

mod our;
mod outer;
mod inner;

const SIZE: usize = 1024 * 1024 * 4;

pub fn run(open_mp_enabled: bool) {
  run_on(open_mp_enabled, 1, SIZE);
  run_on(open_mp_enabled, 2, SIZE);
  run_on(open_mp_enabled, 4, SIZE);
  run_on(open_mp_enabled, 8, SIZE);
  run_on(open_mp_enabled, 16, SIZE);
  run_on(open_mp_enabled, 32, SIZE);
}

fn run_on(open_mp_enabled: bool, array_count: usize, size: usize) {
  utils::affinity_first();
  
  let mut inputs = vec![];
  let mut temps = vec![];
  let mut outputs = vec![];

  for _ in 0 .. array_count {
    inputs.push(create_input(size));
    temps.push(our::create_temp(size));
    outputs.push(unsafe { utils::array::alloc_undef_u64_array(size) });
  }

  utils::affinity_full();

  let name = "Scans (n = ".to_owned() + &size.to_formatted_string(&Locale::en) + ", m = " + &array_count.to_formatted_string(&Locale::en) + ")";
  let title = "m = ".to_owned() + &array_count.to_formatted_string(&Locale::en);
  benchmark_with_title(if array_count == 1 { ChartStyle::SmallWithKey } else { ChartStyle::Small }, 5, &name, &title, || {
    reference_sequential(&inputs, &outputs);
  })
  .parallel("Outer parallelism", ChartLineStyle::SequentialPartition, |thread_count| {
    let task = outer::create_task(&inputs, &outputs);
    Workers::run(thread_count, task);
  })
  .parallel("Inner parallelism", ChartLineStyle::Static, |thread_count| {
    let task = inner::create_task(&inputs, &temps, &outputs);
    Workers::run(thread_count, task);
  })
  .open_mp(open_mp_enabled, "OpenMP", ChartLineStyle::OmpDynamic, "scan", Nesting::Nested, array_count, Some(size))
  .our(|thread_count| {
    let pending = AtomicUsize::new(array_count + 1);
    let task = our::create_initial_task(&inputs, &temps, &outputs, &pending);
    Workers::run(thread_count, task);
  });
}

pub fn reference_sequential(inputs: &[Box<[u64]>], outputs: &[Box<[AtomicU64]>]) -> () {
  for (input, output) in inputs.iter().zip(outputs) {
    scan_sequential(input, 0, output);
  }
  ()
}

pub fn scan_sequential(input: &[u64], initial: u64, output: &[AtomicU64]) -> u64 {
  let mut accumulator = initial;
  assert_eq!(input.len(), output.len());
  for i in 0 .. output.len() {
    accumulator += input[i];
    output[i].store(accumulator, Ordering::Relaxed);
  }
  accumulator
}

pub fn fold_sequential(array: &[u64]) -> u64 {
  let mut accumulator = 0;
  for value in array {
    accumulator += value;
  }
  accumulator
}

fn create_input(size: usize) -> Box<[u64]> {
  (0..size).map(|x| random(x as u64) as u64).collect()
}

fn random(mut seed: u64) -> u32 {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  seed as u32
}
