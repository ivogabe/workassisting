// Only exploit the outer parallelism.
// It performs the different compactions in parallel, but the compactions themself are sequential.
use core::sync::atomic::{Ordering, AtomicU64};
use crate::cases::compact::compact_sequential;
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

#[derive(Copy, Clone)]
struct Data<'a> {
  mask: u64,
  inputs: &'a [Box<[u64]>],
  outputs: &'a [Box<[AtomicU64]>]
}

pub fn create_task(mask: u64, inputs: &[Box<[u64]>], outputs: &[Box<[AtomicU64]>]) -> Task {
  Task::new_dataparallel::<Data>(run, finish, Data{ mask, inputs, outputs }, inputs.len() as u32)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  workassisting_loop!(loop_arguments, |i| {
    compact_sequential(data.mask, &data.inputs[i as usize], &data.outputs[i as usize], 0);
  });
}
fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let _data = unsafe { TaskObject::take_data(task) };
  workers.finish();
}
