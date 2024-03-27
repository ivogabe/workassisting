use core::sync::atomic::{Ordering, AtomicU64};
use crate::cases::compact::{compact_sequential, count_sequential};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;
use super::our::{BLOCK_SIZE, BlockInfo, reset, STATE_AGGREGATE_AVAILABLE, STATE_PREFIX_AVAILABLE};

#[derive(Copy, Clone)]
struct Data<'a> {
  mask: u64,
  inputs: &'a [Box<[u64]>],
  temps: &'a [Box<[BlockInfo]>],
  outputs: &'a [Box<[AtomicU64]>]
}

pub fn create_task(mask: u64, inputs: &[Box<[u64]>], temps: &[Box<[BlockInfo]>], outputs: &[Box<[AtomicU64]>]) -> Task {
  assert!(inputs.len() != 0);
  reset(&temps[0]);
  Task::new_dataparallel::<Data>(run, finish, Data{ mask, inputs, temps, outputs }, ((inputs[0].len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  let mut sequential = true;
  workassisting_loop!(loop_arguments, |block_index| {
    let input = &data.inputs[0];
    let temp = &data.temps[0];
    let output = &data.outputs[0];
    // Local scan
    // reduce-then-scan
    let start = block_index as usize * BLOCK_SIZE as usize;
    let end = ((block_index as usize + 1) * BLOCK_SIZE as usize).min(input.len());

    // Check if we already have an aggregate of the previous block.
    // If that is the case, then we can perform the scan directly.
    // Otherwise we perform a reduce-then-scan over this block.
    let aggregate_start = if !sequential {
      None // Don't switch back from parallel mode to sequential mode
    } else if block_index ==  0 {
      Some(0)
    } else {
      let previous = block_index - 1;
      let previous_state = temp[previous as usize].state.load(Ordering::Acquire);
      if previous_state == STATE_PREFIX_AVAILABLE {
        Some(temp[previous as usize].prefix.load(Ordering::Acquire))
      } else {
        None
      }
    };

    if let Some(aggregate) = aggregate_start {
      let local = compact_sequential(data.mask, &input[start .. end], output, aggregate);
      temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      sequential = false;
      let local = count_sequential(data.mask, &input[start .. end]);
      // Share own local value
      temp[block_index as usize].aggregate.store(local, Ordering::Relaxed);
      temp[block_index as usize].state.store(STATE_AGGREGATE_AVAILABLE, Ordering::Release);

      // Find aggregate
      let mut aggregate = 0;
      let mut previous = block_index - 1;

      loop {
        let previous_state = temp[previous as usize].state.load(Ordering::Acquire);
        if previous_state == STATE_PREFIX_AVAILABLE {
          aggregate = temp[previous as usize].prefix.load(Ordering::Acquire) + aggregate;
          break;
        } else if previous_state == STATE_AGGREGATE_AVAILABLE {
          aggregate = temp[previous as usize].aggregate.load(Ordering::Acquire) + aggregate;
          previous = previous - 1;
        } else {
          // Continue looping until the state of previous block changes.
        }
      }

      // Make aggregate available
      temp[block_index as usize].prefix.store(aggregate + local, Ordering::Relaxed);
      temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
      compact_sequential(data.mask, &input[start .. end], output, aggregate as usize);
    }
  });
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let data = unsafe { TaskObject::take_data(task) };
  if data.inputs.len() == 1 {
    workers.finish();
  } else {
    workers.push_task(create_task(data.mask, &data.inputs[1..], &data.temps[1..], &data.outputs[1..]));
  }
}
