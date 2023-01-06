use core::sync::atomic::AtomicU32;

pub unsafe fn alloc_undef_u32_array(length: usize) -> Box<[AtomicU32]> {
  let mut vector = Vec::with_capacity(length);
  vector.set_len(length);
  vector.into_boxed_slice()
}
