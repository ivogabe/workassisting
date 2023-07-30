#[macro_export]
macro_rules! workassisting_loop {
  ($loop_arguments_expr: expr, |$chunk_index: ident| $body: block) => {
    let mut loop_arguments: LoopArguments = $loop_arguments_expr;
    // Claim work
    let mut chunk_idx = loop_arguments.first_index;

    while chunk_idx < loop_arguments.work_size {
      if chunk_idx == loop_arguments.work_size - 1 {
        // All work is claimed.
        loop_arguments.empty_signal.task_empty();
      }

      // Copy chunk_index to an immutable variable, such that a user of this macro cannot mutate it.
      let $chunk_index = chunk_idx;
      $body

      chunk_idx = loop_arguments.work_index.fetch_add(1, Ordering::Relaxed);
    }
    loop_arguments.empty_signal.task_empty();
  };
}
pub(crate) use workassisting_loop;
