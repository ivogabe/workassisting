// Only exploit the outer parallelism.
// It performs the different scans in parallel, but the scans themself are sequential.
use core::sync::atomic::{Ordering, AtomicU64};
use crate::cases::scan::scan_sequential;
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

#[derive(Copy, Clone)]
struct Data<'a> {
  inputs: &'a [Box<[u64]>],
  outputs: &'a [Box<[AtomicU64]>]
}

pub fn create_task(inputs: &[Box<[u64]>], outputs: &[Box<[AtomicU64]>]) -> Task {
  Task::new_dataparallel::<Data>(run, finish, Data{ inputs, outputs }, inputs.len() as u32)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  workassisting_loop!(loop_arguments, |i| {
    scan_sequential(&data.inputs[i as usize], 0, &data.outputs[i as usize]);
  });
}
fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let _data = unsafe { TaskObject::take_data(task) };
  workers.finish();
}
