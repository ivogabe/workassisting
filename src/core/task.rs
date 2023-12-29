use core::fmt::Debug;
use core::sync::atomic::{ AtomicI32, AtomicU32 };
use core::mem::forget;
use core::ops::{Drop, Deref, DerefMut};
use crate::core::worker::*;

pub struct Task (*mut TaskObject<()>);

#[repr(C)]
pub struct TaskObject<T> {
  // 'work' borrows the TaskObject
  pub(super) work: Option<fn(workers: &Workers, this: *const TaskObject<T>, loop_arguments: LoopArguments) -> ()>,
  // 'finish' takes ownership of the TaskObject
  pub(super) finish: fn(workers: &Workers, this: *mut TaskObject<T>) -> (),
  // The number of active_threads, offset by the tag in the activities array.
  // If this task is present in activities, then:
  //   - active_threads contains - (the number of finished threads), thus non-positive.
  //   - the tag in activities (in AtomicTaggedPtr) contains the number of threads that have started working on this task
  // When a thread removes this task from activities, it will assure that:
  //   - active_threads contains the number of active threads, thus is non-negative
  // When active_threads becomes zero after a decrement:
  //   - the task is not present in activities.
  //   - no thread is still working on this task.
  // Hence we can run the finish function and deallocate the task.
  pub(super) active_threads: AtomicI32,
  pub(super) work_index: AtomicU32,
  pub(super) work_size: u32,
  pub data: T,
}

impl Debug for Task {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    let obj = unsafe { &*self.0 };
    obj.fmt(f)
  }
}

impl<T> Debug for TaskObject<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    let work = self.work.map(|f| f as *const ());
    write!(f, "Task:\n  work {:?}\n  finish {:?}\n size {:?}\n index {:?}\n active threads {:?}", work, self.finish as *const (), self.work_size, self.work_index, self.active_threads)
  }
}

impl Task {
  pub fn new_dataparallel<T: Send + Sync>(
    work: fn(workers: &Workers, data: *const TaskObject<T>, loop_arguments: LoopArguments) -> (),
    finish: fn(workers: &Workers, data: *mut TaskObject<T>) -> (),
    data: T,
    work_size: u32
  ) -> Task {
    let task_box: Box<TaskObject<T>> = Box::new(TaskObject{
      work: Some(work),
      finish,
      work_size,
      active_threads: AtomicI32::new(0),
      work_index: AtomicU32::new(1),
      data
    });
    Task(Box::into_raw(task_box) as *mut TaskObject<()>)
  }

  pub fn new_single<T: Send + Sync>(
    function: fn(workers: &Workers, data: *mut TaskObject<T>) -> (),
    data: T
  ) -> Task {
    let task_box: Box<TaskObject<T>> = Box::new(TaskObject{
      work: None,
      finish: function,
      work_size: 0,
      active_threads: AtomicI32::new(0),
      work_index: AtomicU32::new(0),
      data
    });
    Task(Box::into_raw(task_box) as *mut TaskObject<()>)
  }

  // The caller should assure that the object is properly deallocated.
  // This can be done by calling Task::from_raw.
  pub fn into_raw(self) -> *mut TaskObject<()> {
    let ptr = self.0;
    forget(self); // Don't run drop() on self, as that would deallocate the TaskObject
    ptr
  }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Drop for Task {
  fn drop(&mut self) {
    // We cannot drop the TaskObject<T> here, as we don't know the type argument T here.
    // We assume that the TaskObject is passed to Workers; that will handle the deallocation of the TaskObject.
    println!("Warning: TaskObject not cleared. Make sure that all constructed Tasks are also passed to Workers.");
  }
}

impl Deref for Task {
  type Target = TaskObject<()>;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.0 }
  }
}

impl DerefMut for Task {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *self.0 }
  }
}

impl<T> TaskObject<T> {
  // Safety: caller should guarantee that the TaskObject outlives lifetime 'a.
  pub unsafe fn get_data<'a>(task: *const TaskObject<T>) -> &'a T {
    unsafe { &(*task).data }
  }

  pub unsafe fn take_data<'a>(task: *mut TaskObject<T>) -> T {
    unsafe { Box::from_raw(task) }.data
  }
}

pub struct LoopArguments<'a> {
  pub work_size: u32,
  pub work_index: &'a AtomicU32,
  pub empty_signal: EmptySignal<'a>,
  pub first_index: u32,
}
