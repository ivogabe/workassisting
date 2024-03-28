use std::io;
use std::mem;

pub unsafe fn unsafe_spawn_on<F: FnOnce()>(core: usize, p: Box<F>) -> io::Result<libc::pthread_t> {
  let p = Box::into_raw(Box::new(p));
  let mut native: libc::pthread_t = mem::zeroed();
  let mut attr: libc::pthread_attr_t = mem::zeroed();
  assert_eq!(libc::pthread_attr_init(&mut attr), 0);

  /* #[cfg(not(target_os = "espidf"))]
  {
      let stack_size = cmp::max(stack, min_stack_size(&attr));

      match libc::pthread_attr_setstacksize(&mut attr, stack_size) {
          0 => {}
          n => {
              assert_eq!(n, libc::EINVAL);
              // EINVAL means |stack_size| is either too small or not a
              // multiple of the system page size. Because it's definitely
              // >= PTHREAD_STACK_MIN, it must be an alignment issue.
              // Round up to the nearest page and try again.
              let page_size = os::page_size();
              let stack_size =
                  (stack_size + page_size - 1) & (-(page_size as isize - 1) as usize - 1);
              assert_eq!(libc::pthread_attr_setstacksize(&mut attr, stack_size), 0);
          }
      };
  } */

  let mut cpuset: libc::cpu_set_t = mem::zeroed();
  libc::CPU_SET(core, &mut cpuset);
  libc::pthread_attr_setaffinity_np(&mut attr, std::mem::size_of::<libc::cpu_set_t>(), &cpuset);

  let ret = libc::pthread_create(&mut native, &attr, thread_start::<F>, p as *mut _);
  // Note: if the thread creation fails and this assert fails, then p will
  // be leaked. However, an alternative design could cause double-free
  // which is clearly worse.
  assert_eq!(libc::pthread_attr_destroy(&mut attr), 0);

  return if ret != 0 {
      // The thread failed to start and as a result p was not consumed. Therefore, it is
      // safe to reconstruct the box so that it gets deallocated.
      drop(Box::from_raw(p));
      Err(io::Error::from_raw_os_error(ret))
  } else {
      Ok(native)
  };
}

extern "C" fn thread_start<F: FnOnce()>(main: *mut libc::c_void) -> *mut libc::c_void {
  unsafe {
      // Next, set up our stack overflow handler which may get triggered if we run
      // out of stack.
      // let _handler = stack_overflow::Handler::new();
      // Finally, let's run some code.
      Box::from_raw(main as *mut Box<F>)();
  }
  std::ptr::null_mut()
}
