use std::sync::atomic::{AtomicUsize, Ordering};
use std::marker::PhantomData;

// The number of unused bits, seen from the most-significant side
const UNUSED_MOST_SIGNIFICANT_BITS: u32 = 16;

#[repr(transparent)]
pub struct AtomicTaggedPtr<T> {
  phantom: PhantomData<*mut T>,
  value: AtomicUsize,
}

unsafe impl<T: Send> Send for AtomicTaggedPtr<T> {}
unsafe impl<T: Send> Sync for AtomicTaggedPtr<T> {}

#[allow(dead_code)]
impl<T> AtomicTaggedPtr<T> {
  pub fn new(value: TaggedPtr<T>) -> AtomicTaggedPtr<T> {
    AtomicTaggedPtr { phantom: PhantomData, value: AtomicUsize::new(value.value) }
  }

  pub fn load(&self, order: Ordering) -> TaggedPtr<T> {
    let result = self.value.load(order);
    TaggedPtr::from_usize(result)
  }

  pub fn store(&self, new: TaggedPtr<T>, order: Ordering) {
    self.value.store(new.value, order);
  }

  pub fn compare_exchange(&self, current: TaggedPtr<T>, new: TaggedPtr<T>, success: Ordering, failure: Ordering)
      -> Result<TaggedPtr<T>, TaggedPtr<T>> {
    match self.value.compare_exchange(current.value, new.value, success, failure) {
      Ok(x) => Ok(TaggedPtr::from_usize(x)),
      Err(x) => Err(TaggedPtr::from_usize(x))
    }
  }

  pub fn compare_exchange_weak(&self, current: TaggedPtr<T>, new: TaggedPtr<T>, success: Ordering, failure: Ordering)
      -> Result<TaggedPtr<T>, TaggedPtr<T>> {
    match self.value.compare_exchange_weak(current.value, new.value, success, failure) {
      Ok(x) => Ok(TaggedPtr::from_usize(x)),
      Err(x) => Err(TaggedPtr::from_usize(x))
    }
  }

  pub fn swap(&self, new: TaggedPtr<T>, order: Ordering) -> TaggedPtr<T> {
    let result = self.value.swap(new.value, order);
    TaggedPtr::from_usize(result)
  }

  pub fn fetch_add_tag(&self, tag: usize, order: Ordering) -> TaggedPtr<T> {
    let result = self.value.fetch_add(tag, order);
    TaggedPtr::from_usize(result)
  }
}

#[repr(transparent)]
pub struct TaggedPtr<T> {
  phantom: PhantomData<*mut T>,
  value: usize,
}

impl<T> TaggedPtr<T> {
  fn from_usize(value: usize) -> TaggedPtr<T> {
    TaggedPtr { phantom: PhantomData, value }
  }

  pub fn new(ptr: *const T, tag: usize) -> TaggedPtr<T> {
    // Assert that the tag does not require more bits than we have available
    assert_eq!(tag >> UNUSED_MOST_SIGNIFICANT_BITS, 0);
    TaggedPtr { phantom: PhantomData, value: ((ptr as usize) << UNUSED_MOST_SIGNIFICANT_BITS) | tag }
  }

  pub fn ptr(self) -> *const T {
    // Note that we use a sign-extending shift, see
    // https://stackoverflow.com/questions/16198700/using-the-extra-16-bits-in-64-bit-pointers
    ((self.value as isize) >> UNUSED_MOST_SIGNIFICANT_BITS) as *const T
  }

  pub fn tag(self) -> usize {
    let mask = (1 << UNUSED_MOST_SIGNIFICANT_BITS) - 1;
    self.value & mask
  }
}

impl<T> Clone for TaggedPtr<T> {
  fn clone(&self) -> Self {
    Self { phantom: PhantomData, value: self.value }
  }
}

impl<T> Copy for TaggedPtr<T> {}
