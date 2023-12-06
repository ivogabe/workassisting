use core::fmt::Debug;
use core::mem;
use core::sync::atomic::{ AtomicI32, AtomicU32 };
use std::alloc::Layout;
use core::mem::forget;
use core::ops::{Drop, Deref, DerefMut};
use crate::core::worker::*;

pub struct Task (*mut TaskObject);

pub struct TaskObject {
  pub function: fn(workers: &Workers, data: &(), loop_arguments: LoopArguments) -> (),
  pub continuation: fn(workers: &Workers, data: &()) -> (),
  pub data_offset: usize,
  // The number of active_threads, offset by the tag in the activities array.
  // If this task is present in activities, then:
  //   - active_threads contains - (the number of finished threads), thus non-positive.
  //   - the tag in activities (in AtomicTaggedPtr) contains the number of threads that have started working on this task
  // When a thread removes this task from activities, it will assure that:
  //   - active_threads contains the number of active threads, thus is non-negative
  // When active_threads becomes zero after a decrement:
  //   - the task is not present in activities.
  //   - no thread is still working on this task.
  // Hence we can run the continuation function and deallocate the task.
  pub active_threads: AtomicI32,
  pub work_index: AtomicU32,
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
    write!(f, "Task:\n  function {:?}\n  continuation {:?}\n size {:?}\n index {:?}\n active threads {:?}", self.function as *const (), self.continuation as *const (), self.work_size, self.work_index, self.active_threads)
  }
}

impl Task {
  pub fn new_dataparallel<T: Send + Sync>(
    function: fn(workers: &Workers, data: &T, loop_arguments: LoopArguments) -> (),
    continuation: fn(workers: &Workers, data: &T) -> (),
    data: T,
    work_size: u32
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
        work_size,
        active_threads: AtomicI32::new(0),
        work_index: AtomicU32::new(1),
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
        active_threads: AtomicI32::new(0),
        work_index: AtomicU32::new(0),
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
  pub work_size: u32,
  pub work_index: &'a AtomicU32,
  pub empty_signal: EmptySignal<'a>,
  pub first_index: u32,
}
