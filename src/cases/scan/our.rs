use core::sync::atomic::{Ordering, AtomicU64, AtomicUsize};
use crate::cases::scan::{scan_sequential, fold_sequential};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

pub const BLOCK_SIZE: u64 = 1024 * 4;

#[derive(Copy, Clone)]
struct InitialData<'a> {
  inputs: &'a [Box<[u64]>],
  temps: &'a [Box<[BlockInfo]>],
  outputs: &'a [Box<[AtomicU64]>],
  pending: &'a AtomicUsize
}

pub fn create_initial_task(inputs: &[Box<[u64]>], temps: &[Box<[BlockInfo]>], outputs: &[Box<[AtomicU64]>], pending: &AtomicUsize) -> Task {
  if inputs.len() == 1 {
    pending.store(1, Ordering::Relaxed);
    create_task(&inputs[0], &temps[0], &outputs[0], pending)
  } else {
    Task::new_dataparallel::<InitialData>(initial_run, initial_finish, InitialData{ inputs, temps, outputs, pending }, inputs.len() as u32)
  }
}

fn initial_run(workers: &Workers, task: *const TaskObject<InitialData>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  workassisting_loop!(loop_arguments, |i| {
    workers.push_task(create_task(&data.inputs[i as usize], &data.temps[i as usize], &data.outputs[i as usize], data.pending));
  });
}
fn initial_finish(workers: &Workers, task: *mut TaskObject<InitialData>) {
  let data = unsafe { TaskObject::take_data(task) };
  if data.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
    workers.finish();
  }
}

#[derive(Copy, Clone)]
struct Data<'a> {
  input: &'a [u64],
  temp: &'a [BlockInfo],
  output: &'a [AtomicU64],
  pending: &'a AtomicUsize
}

pub struct BlockInfo {
  pub state: AtomicU64,
  pub aggregate: AtomicU64,
  pub prefix: AtomicU64
}

pub const STATE_INITIALIZED: u64 = 0;
pub const STATE_AGGREGATE_AVAILABLE: u64 = 1;
pub const STATE_PREFIX_AVAILABLE: u64 = 2;

pub fn create_temp(size: usize) -> Box<[BlockInfo]> {
  (0 .. (size as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE).map(|_| BlockInfo{
    state: AtomicU64::new(STATE_INITIALIZED), aggregate: AtomicU64::new(0), prefix: AtomicU64::new(0)
  }).collect()
}

pub fn reset(temp: &[BlockInfo]) {
  for i in 0 .. temp.len() {
    temp[i].state.store(STATE_INITIALIZED, Ordering::Relaxed);
    temp[i].aggregate.store(0, Ordering::Relaxed);
    temp[i].prefix.store(0, Ordering::Relaxed);
  }
}

pub fn create_task(input: &[u64], temp: &[BlockInfo], output: &[AtomicU64], pending: &AtomicUsize) -> Task {
  reset(temp);
  Task::new_dataparallel::<Data>(run, finish, Data{ input, temp, output, pending }, ((input.len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  let mut sequential = true;
  workassisting_loop!(loop_arguments, |block_index| {
    // Local scan
    // reduce-then-scan
    let start = block_index as usize * BLOCK_SIZE as usize;
    let end = ((block_index as usize + 1) * BLOCK_SIZE as usize).min(data.input.len());

    // Check if we already have an aggregate of the previous block.
    // If that is the case, then we can perform the scan directly.
    // Otherwise we perform a reduce-then-scan over this block.
    let aggregate_start = if !sequential {
      None // Don't switch back from parallel mode to sequential mode
    } else if block_index ==  0 {
      Some(0)
    } else {
      let previous = block_index - 1;
      let previous_state = data.temp[previous as usize].state.load(Ordering::Acquire);
      if previous_state == STATE_PREFIX_AVAILABLE {
        Some(data.temp[previous as usize].prefix.load(Ordering::Acquire))
      } else {
        None
      }
    };

    if let Some(aggregate) = aggregate_start {
      let local = scan_sequential(&data.input[start .. end], aggregate, &data.output[start .. end]);
      data.temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      sequential = false;
      let local = fold_sequential(&data.input[start .. end]);
      // Share own local value
      data.temp[block_index as usize].aggregate.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_AGGREGATE_AVAILABLE, Ordering::Release);

      // Find aggregate
      let mut aggregate = 0;
      let mut previous = block_index - 1;

      loop {
        let previous_state = data.temp[previous as usize].state.load(Ordering::Acquire);
        if previous_state == STATE_PREFIX_AVAILABLE {
          aggregate = data.temp[previous as usize].prefix.load(Ordering::Acquire) + aggregate;
          break;
        } else if previous_state == STATE_AGGREGATE_AVAILABLE {
          aggregate = data.temp[previous as usize].aggregate.load(Ordering::Acquire) + aggregate;
          previous = previous - 1;
        } else {
          // Continue looping until the state of previous block changes.
        }
      }

      // Make aggregate available
      data.temp[block_index as usize].prefix.store(aggregate + local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
      scan_sequential(&data.input[start .. end], aggregate, &data.output[start .. end]);
    }
  });
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let data = unsafe { TaskObject::take_data(task) };
  if data.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
    workers.finish();
  }
}
