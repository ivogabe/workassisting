use core::panic;
use core::sync::atomic::AtomicU64;
use crate::utils::matrix::SquareMatrix;
use crate::core::worker::Workers;
use crate::utils::benchmark::{benchmark_with_title, ChartStyle, ChartLineStyle};
use num_format::{Locale, ToFormattedString};

pub mod our;
pub mod workstealing;

pub fn run(openmp_enabled: bool) {
  test("sequential", |mut matrix| {
    sequential(&mut matrix);
    matrix
  });
  test("sequential-tiled", |mut matrix| {
    sequential_tiled(&mut matrix);
    matrix
  });
  test("workstealing", |matrix| {
    let pending = AtomicU64::new(0);
    let mut matrices = vec![(matrix, AtomicU64::new(0), AtomicU64::new(0))];
    workstealing::run(&matrices, &pending, 2);
    let result = matrices.pop().unwrap();
    result.0
  });
  test("workassisting", |matrix| {
    let pending = AtomicU64::new(0);
    let mut matrices = vec![(matrix, AtomicU64::new(0), AtomicU64::new(0))];
    Workers::run(2, our::create_task(&matrices, &pending));
    let result = matrices.pop().unwrap();
    result.0
  });

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

  let name = "LUD (n = ".to_owned() + &size.to_formatted_string(&Locale::en) + ", m = " + &matrix_count.to_formatted_string(&Locale::en) + ")";
  let title = "m = ".to_owned() + &matrix_count.to_formatted_string(&Locale::en);
  benchmark_with_title(if matrix_count == 1 { ChartStyle::SmallWithKey } else { ChartStyle::Small }, 9, &name, &title, || {
    for i in 0 .. matrix_count {
      input.copy_to(&mut matrices[i].0);
      sequential_tiled(&mut matrices[i].0);
    }
  })
  .sequential("Sequential without tiling", || {
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
  .open_mp_lud(openmp_enabled, "OpenMP (loops)", false, ChartLineStyle::OmpDynamic, &filename(size), matrix_count)
  .open_mp_lud(openmp_enabled, "OpenMP (tasks)", true, ChartLineStyle::OmpTask, &filename(size), matrix_count)
  .our(|thread_count| {
    for i in 0 .. matrix_count {
      input.copy_to(&mut matrices[i].0);
    }
    Workers::run(thread_count, our::create_task(&matrices, &pending));
  });
}

fn test<F: FnOnce(SquareMatrix) -> SquareMatrix>(name: &str, f: F) {
  let matrix = parse_input(512);

  let input = matrix.clone();

  let result = f(matrix);

  let u = result.upper_triangle_with_diagonal();
  let l = result.lower_triangle_with_1_diagonal();
  let error = compute_error(&input, &l, &u);
  if error > 10.0 {
    panic!("Large (rounding?) error ({}) in result of LU decomposition in {} implementation. The implementation is probably incorrect.", name, error);
  } else if error.is_nan() {
    panic!("Result of {} implementation contains NaN", name);
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

fn sequential_tiled(matrix: &mut SquareMatrix) {
  const TILE_SIZE: usize = 32;
  for offset in (0 .. matrix.size()).step_by(TILE_SIZE) {
    // Handle the tile on the diagonal, starting at (offset, offset).
    diagonal_tile(offset, matrix);

    // Handle the left and top border of this iteration
    let chunks = (matrix.size() - offset - TILE_SIZE) / TILE_SIZE;
    {
      let mut temp = Align([0.0; TILE_SIZE * TILE_SIZE]);
      border_init(offset, matrix, &mut temp);
      for chunk_index in 0 .. chunks {
        border_left_chunk::<TILE_SIZE>(offset, matrix, &mut temp, chunk_index);
      }
      for chunk_index in 0 .. chunks {
        border_top_chunk::<TILE_SIZE>(offset, matrix, &mut temp, chunk_index);
      }
    }

    // Handle the interior of this iteration
    {
      let mut temp_top = Align([0.0; TILE_SIZE * TILE_SIZE]);
      let mut sum = Align([0.0; TILE_SIZE]);
      let mut temp_index = 0;

      for chunk_index in 0 .. chunks * chunks {
        interior_chunk::<TILE_SIZE, TILE_SIZE>
          (offset, chunks, matrix, &mut temp_index, &mut temp_top.0, &mut sum.0, chunk_index);
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

pub const TILE_SIZE: usize = 32;
// Handles the tile on the diagonal, at the start of a new iteration.
#[inline(always)]
pub fn diagonal_tile(offset: usize, matrix: &SquareMatrix) {
  for i in 0 .. TILE_SIZE {
    for j in i .. TILE_SIZE {
      for k in 0 .. i {
        matrix.write((offset + i, offset + j), matrix[(offset + i, offset + j)] - matrix[(offset + i, offset + k)] * matrix[(offset + k, offset + j)]);
      }
    }

    let temp = 1.0 / matrix[(offset + i, offset + i)];
    for j in i + 1 .. TILE_SIZE {
      for k in 0 .. i {
        matrix.write((offset + j, offset + i), matrix[(offset + j, offset + i)] - matrix[(offset + j, offset + k)] * matrix[(offset + k, offset + i)]);
      }
      matrix.write((offset + j, offset + i), matrix[(offset + j, offset + i)] * temp);
    }
  }
}

#[repr(C)]
#[repr(align(64))]
pub struct Align<T>(T);

// Initialization work for border_left_chunk and border_top_chunk.
#[inline(always)]
pub fn border_init(offset: usize, matrix: &SquareMatrix, temp: &mut Align<[f32; TILE_SIZE * TILE_SIZE]>) {
  // Copy part of the matrix to 'temp'
  for i in 0 .. TILE_SIZE {
    let matrix_slice = unsafe { *matrix.slice32(offset + i, offset).get() };
    for j in 0 .. TILE_SIZE {
      temp.0[i * TILE_SIZE + j] = matrix_slice.0[j];
    }
  }
}

#[inline(always)]
pub fn border_left_chunk<const BORDER_BLOCK_SIZE: usize>(offset: usize, matrix: &SquareMatrix, temp: &Align<[f32; TILE_SIZE * TILE_SIZE]>, chunk_index: usize) {
  let i_global = offset + TILE_SIZE + BORDER_BLOCK_SIZE * chunk_index;
  let j_global = offset;
  for j in 0 .. TILE_SIZE {
    for i in 0 .. BORDER_BLOCK_SIZE {
      let mut sum = 0.0;
      let slice = unsafe { *matrix.slice32(i_global + i, j_global).get() };
      for k in 0 .. j {
        sum += slice.0[k] * temp.0[TILE_SIZE * k + j];
      }
      matrix.write(
        (i_global + i, j_global + j),
        (matrix[(i_global + i, j_global + j)] - sum) / temp.0[j * TILE_SIZE + j]
      );
    }
  }
}

#[inline(always)]
pub fn border_top_chunk<const BORDER_BLOCK_SIZE: usize>(offset: usize, matrix: &SquareMatrix, temp: &Align<[f32; TILE_SIZE * TILE_SIZE]>, chunk_index: usize) {
  let i_global = offset;
  let j_global = offset + TILE_SIZE + BORDER_BLOCK_SIZE * (chunk_index as usize);
  for j in 0 .. TILE_SIZE {
    for i in 1 .. TILE_SIZE {
      let mut sum = 0.0;
      let temp_slice = &temp.0[i * TILE_SIZE .. i * TILE_SIZE + i];
      for k in 0 .. i {
        sum += temp_slice[k] * matrix[(i_global + k, j_global + j)];
      }
      matrix.write((i_global + i, j_global + j), matrix[(i_global + i, j_global + j)] - sum);
    }
  }
}

#[inline(always)]
pub fn interior_chunk<const INNER_BLOCK_SIZE_ROWS: usize, const INNER_BLOCK_SIZE_COLUMNS: usize>
    (offset: usize, rows: usize, matrix: &SquareMatrix, temp_index: &mut usize, temp_top: &mut [f32], sum: &mut [f32], chunk_index: usize) {
  let i_global = offset + TILE_SIZE + INNER_BLOCK_SIZE_ROWS * (chunk_index as usize % rows);
  let j_global = offset + TILE_SIZE + INNER_BLOCK_SIZE_COLUMNS * (chunk_index as usize / rows);

  if *temp_index != j_global {
    for i in 0 .. TILE_SIZE {
      // Safety: no other thread will write to this cell.
      let matrix_slice = unsafe { *matrix.slice32(offset + i, j_global).get() };
      for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
        temp_top[i * INNER_BLOCK_SIZE_COLUMNS + j] = matrix_slice.0[j];
      }
    }
    *temp_index = j_global;
  }

  for i in 0 .. INNER_BLOCK_SIZE_ROWS {
    let left_slice = unsafe { &*matrix.slice32(i_global + i, offset).get() };
    for k in 0 .. TILE_SIZE {
      let left = left_slice.0[k];
      let top_slice = &temp_top[INNER_BLOCK_SIZE_COLUMNS * k .. INNER_BLOCK_SIZE_COLUMNS * k + INNER_BLOCK_SIZE_COLUMNS];
      for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
        sum[j] += left * top_slice[j];
      }
    }

    let matrix_slice_cell = matrix.slice32(i + i_global, j_global);
    // Safety: only this thread will read and write this part of the matrix at this stage.
    let mut matrix_slice = unsafe { *matrix_slice_cell.get() };
    for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
      matrix_slice.0[j] -= sum[j];
      sum[j] = 0.0;
    }
    unsafe {
      *matrix_slice_cell.get() = matrix_slice;
    }
  }
}
