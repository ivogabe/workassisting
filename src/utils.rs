pub mod array;
pub mod benchmark;
pub mod deque_stealer;
pub mod rayon;
pub mod loops;
pub mod matrix;
pub mod ptr;
pub mod thread_pinning;
pub mod thread;

pub fn affinity_full() {
  affinity::set_thread_affinity(thread_pinning::AFFINITY_MAPPING).unwrap();
}

pub fn affinity_first() {
  affinity::set_thread_affinity([0]).unwrap();
}
