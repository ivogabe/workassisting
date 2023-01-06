#[macro_export]
macro_rules! workstealing_loop {
  ($loop_arguments_expr: expr, |$block_index: ident| $body: block) => {
    let mut loop_arguments: LoopArguments = $loop_arguments_expr;
    // Claim work
    let mut block_index = loop_arguments.first_index;

    while block_index < loop_arguments.workstealing_size {
      if block_index == loop_arguments.workstealing_size - 1 {
        // All work is claimed.
        loop_arguments.empty_signal.task_empty();
      }

      // Copy block_index to an immutable variable, such that a user of this macro cannot mutate it.
      let $block_index = block_index;
      $body

      block_index = loop_arguments.workstealing_index.fetch_add(1, Ordering::Relaxed);
    }
    loop_arguments.empty_signal.task_empty();
  };
}
pub(crate) use workstealing_loop;
