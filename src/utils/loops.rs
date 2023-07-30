/// Creates a loop that runs from $start to the minimum of $end1 and $end2 (exclusive).
/// The loop code is duplicated, to allow the compiler to specialize for a specific size implied by $end1 or $end2.
#[macro_export]
macro_rules! loop_fixed_size {
  ($index: ident in $start: expr, $end1: expr, $end2: expr, $body: block) => {
    if $end1 < $end2 {
      for $index in $start .. $end1 {
        $body
      }
    } else {
      for $index in $start .. $end2 {
        $body
      }
    }
  }
}
pub(crate) use loop_fixed_size;

/// Based on $condition, assigns either $when_true or $when_false to $var and then executes $chunk.
/// The code of $chunk is duplicated, to allow the compiler to specialize the code for the value
/// of $var.
#[macro_export]
macro_rules! specialize_if {
  ($condition: expr, $when_true: expr, $when_false: expr, |$var: ident| $chunk: block) => {
    if $condition {
      let $var = $when_true;
      $chunk
    } else {
      let $var = $when_false;
      $chunk
    }
  }
}
