use core::panic;
use core::sync::atomic::AtomicU64;
use crate::utils::matrix::SquareMatrix;
use crate::core::worker::Workers;
use crate::utils::benchmark::{benchmark, ChartStyle};
use num_format::{Locale, ToFormattedString};

pub mod our;
pub mod workstealing;

pub fn run(openmp_enabled: bool) {
  test();
  run_on(openmp_enabled, 256, 1);
  run_on(openmp_enabled, 256, 2);
  run_on(openmp_enabled, 512, 1);
  run_on(openmp_enabled, 512, 2);
  run_on(openmp_enabled, 512, 4);
  run_on(openmp_enabled, 512, 8);
  run_on(openmp_enabled, 512, 16);
  run_on(openmp_enabled, 512, 32);
}

fn run_on(openmp_enabled: bool, size: usize, matrix_count: usize) {
  let input = parse_input(size);

  let mut matrices: Vec<(SquareMatrix, AtomicU64, AtomicU64)> = (0 .. matrix_count).map(|_| {
    (SquareMatrix::new(size), AtomicU64::new(0), AtomicU64::new(0))
  }).collect();

  let pending = AtomicU64::new(0);

  let name = "LU (n = ".to_owned() + &size.to_formatted_string(&Locale::en) + ", m = " + &matrix_count.to_formatted_string(&Locale::en) + ")";
  benchmark(ChartStyle::WithKey, &name, || {
    for i in 0 .. matrix_count {
      input.copy_to(&mut matrices[i].0);
      sequential(&mut matrices[i].0);
    }
  })
  .work_stealing(|thread_count| {
    for i in 0 .. matrix_count {
      input.copy_to(&mut matrices[i].0);
    }
    workstealing::run(&matrices, &pending, thread_count);
  })
  .open_mp_lud(openmp_enabled, "OpenMP", 5, &filename(size), matrix_count)
  .our(|thread_count| {
    for i in 0 .. matrix_count {
      input.copy_to(&mut matrices[i].0);
    }
    Workers::run(thread_count, our::create_task(&matrices, &pending));
  });
}

fn test() {
  let matrix = parse_input(512);

  let input = matrix.clone();

  // println!("A = {:?}", matrix);

  let pending = AtomicU64::new(0);
  let matrices = vec![(matrix, AtomicU64::new(0), AtomicU64::new(0))];
  Workers::run(32, our::create_task(&matrices, &pending));

  let u = matrices[0].0.upper_triangle_with_diagonal();
  // println!("U = {:?}", u);
  let l = matrices[0].0.lower_triangle_with_1_diagonal();
  if compute_error(&input, &l, &u) > 10.0 {
    panic!("Large (rounding?) error in result of LU decomposition. The implementation is probably incorrect.");
  }
}

fn sequential(matrix: &mut SquareMatrix) {
  for i in 0 .. matrix.size() {
    for j in i + 1 .. matrix.size() {
      matrix[(j, i)] = matrix[(j, i)] / matrix[(i, i)];

      for k in i + 1 .. matrix.size() {
        matrix[(j, k)] = matrix[(j, k)] - matrix[(j, i)] * matrix[(i, k)];
      }
    }
  }
}

fn compute_error(input: &SquareMatrix, l: &SquareMatrix, u: &SquareMatrix) -> f32 {
  let lu = l * u;
  let mut sum = 0.0;
  for column in 0 .. lu.size() {
    for row in 0 .. lu.size() {
      sum += (input[(row, column)] - lu[(row, column)]).abs();
    }
  }
  sum
}

fn filename(size: usize) -> String {
  "./rodinia_3.1/data/lud/".to_owned() + &size.to_string() + ".dat"
}

fn parse_input(size: usize) -> SquareMatrix {
  let mut matrix = SquareMatrix::new(size);

  let content = std::fs::read_to_string(filename(size)).expect("Data file with input matrix not found");

  let size_str = size.to_string() + "\n";
  if !content.starts_with(&size_str) {
    panic!("Data file should start with the input size on the first line");
  }

  for (i, line) in content.lines().skip(1).enumerate() {
    for (j, word) in line.split_inclusive(' ').enumerate() {
      matrix[(i, j)] = word.trim().parse().expect(&("Could not parse floating point number: ".to_owned() + word));
    }
  }
  matrix
}
