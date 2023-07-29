mod core;
mod cases;
mod utils;

fn main() {
  let open_mp_enabled = true;
  cases::lu::run(open_mp_enabled);
  cases::quicksort::run(open_mp_enabled);
  cases::prime::run(open_mp_enabled);
  cases::sum_array::run(open_mp_enabled);
  cases::sum_function::run(open_mp_enabled);
}

