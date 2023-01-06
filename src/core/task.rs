use core::fmt::Debug;
use core::mem;
use core::sync::atomic::{ AtomicU32, AtomicU64, Ordering };
use core::mem::ManuallyDrop;
use std::alloc::Layout;
use core::mem::forget;
use core::ops::{Drop, Deref, DerefMut};
use crate::core::worker::*;

pub struct Task (*mut TaskObject);

pub struct TaskObject {
  pub function: fn(workers: &Workers, data: &(), loop_arguments: LoopArguments) -> (),
  pub continuation: fn(workers: &Workers, data: &()) -> (),
  pub data_offset: usize,
  pub counters: Counters, // active_threads and work_index
  pub work_size: u32,
  pub layout: Layout, // The layout of the TaskObject extended with the data. Needed to deallocate them
}

impl Debug for Task {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    let obj = unsafe { &*self.0 };
    obj.fmt(f)
  }
}

impl Debug for TaskObject {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    write!(f, "Task:\n  function {:?}\n  continuation {:?}\n size {:?}\n index {:?}\n active threads {:?}", self.function as *const (), self.continuation as *const (), self.work_size, self.counters.work_index(), self.counters.active_threads())
  }
}

impl Task {
  pub fn new_dataparallel<T: Send + Sync>(
    function: fn(workers: &Workers, data: &T, loop_arguments: LoopArguments) -> (),
    continuation: fn(workers: &Workers, data: &T) -> (),
    data: T,
    workstealing_size: u32
  ) -> Task {
    let layout_task = Layout::new::<TaskObject>();
    let layout_data = Layout::new::<T>();
    let (layout, data_offset) = layout_task.extend(layout_data).expect("Overflow when constructing allocation layout of task");

    let memory = unsafe { std::alloc::alloc(layout) };
    let task_ptr = memory as *mut TaskObject;
    let data_ptr = unsafe { memory.add(data_offset) } as *mut T;

    unsafe {
      *task_ptr = TaskObject{
        function: mem::transmute(function),
        continuation: mem::transmute(continuation),
        data_offset,
        work_size: workstealing_size,
        counters: Counters::new(1, 1),
        layout
      };
      *data_ptr = data;
    }
    Task(task_ptr)
  }

  pub fn new_single<T: Send + Sync>(
    function: fn(workers: &Workers, data: &T) -> (),
    data: T
  ) -> Task {
    let layout_task = Layout::new::<TaskObject>();
    let layout_data = Layout::new::<T>();
    let (layout, data_offset) = layout_task.extend(layout_data).expect("Overflow when constructing allocation layout of task");

    let memory = unsafe { std::alloc::alloc(layout) };
    let task_ptr = memory as *mut TaskObject;
    let data_ptr = unsafe { memory.add(data_offset) } as *mut T;

    unsafe {
      *task_ptr = TaskObject{
        function: no_work,
        continuation: mem::transmute(function),
        data_offset,
        work_size: 0,
        counters: Counters::new(1, 0),
        layout
      };
      *data_ptr = data;
    }
    Task(task_ptr)
  }

  // This is unsafe, as the caller should now assure that the object is properly deallocated.
  // This can be done by calling Task::from_raw.
  pub unsafe fn into_raw(self) -> *mut TaskObject {
    let ptr = self.0;
    forget(self); // Don't run drop() on self, as that would deallocate the TaskObject
    ptr
  }

  // This is unsafe, as the type system doesn't guarantee that the pointer points to a proper TaskObject.
  pub unsafe fn from_raw(ptr: *mut TaskObject) -> Task {
    Task(ptr)
  }

  pub unsafe fn ptr_data(ptr: *const TaskObject) -> *const () {
    unsafe {
      (ptr as *const u8).add((*ptr).data_offset) as *const ()
    }
  }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Drop for Task {
  fn drop(&mut self) {
    // println!("Deallocate task");
    unsafe {
      std::alloc::dealloc(self.0 as *mut u8, (*self.0).layout);
    }
  }
}

impl Deref for Task {
  type Target = TaskObject;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.0 }
  }
}


impl DerefMut for Task {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *self.0 }
  }
}

fn no_work(_workers: &Workers, _data: &(), _loop_arguments: LoopArguments) {
  println!("Should be unreachable!");
}

pub struct LoopArguments<'a> {
  pub workstealing_size: u32,
  pub workstealing_index: &'a AtomicU32,
  pub empty_signal: EmptySignal<'a>,
  pub first_index: u32,
}

#[cfg(target_endian = "big")]
const COUNTER_IDX_THREADS: usize = 0;
#[cfg(target_endian = "little")]
const COUNTER_IDX_THREADS: usize = 1;

const COUNTER_IDX_WORK: usize = 1 - COUNTER_IDX_THREADS;

pub union Counters {
  // In the 32 most significant bits, we store the number of active threads.
  // In the 32 least significant bits, we store the index of the next workstealing item.
  //
  // Items in a union need to implement Copy, which Atomics do not have.
  // We wrap it in ManuallyDrop as a work-around.
  // Atomics don't have a Drop instance, so it is safe to use ManuallyDrop.
  single: ManuallyDrop<AtomicU64>,
  separate: [ManuallyDrop<AtomicU32>; 2]
}

impl Counters {
  pub fn new(active_threads: u32, work_index: u32) -> Counters {
    Counters{ single: ManuallyDrop::new(AtomicU64::new(((active_threads as u64) << 32) | work_index as u64)) }
  }

  pub fn active_threads(&self) -> &AtomicU32 {
    unsafe { &self.separate[COUNTER_IDX_THREADS] }
  }
  pub fn work_index(&self) -> &AtomicU32 {
    unsafe { &self.separate[COUNTER_IDX_WORK] }
  }
  pub fn combined(&self) -> &AtomicU64 {
    unsafe { &self.single }
  }

  pub fn fetch_add(&self, active_threads: u32, work_index: u32, order: Ordering) -> (u32, u32) {
    let old = self.combined().fetch_add(((active_threads as u64) << 32) | work_index as u64, order);
    ((old >> 32) as u32, (old & 0xFFFFFFFF) as u32)
  }
}
