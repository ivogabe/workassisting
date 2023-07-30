// Implementation of LU decomposition.
// This implementation is mainly based on the OpenMP implementation from Rodinia.
// Source: /rodinia_3.1/openmp/lud/omp/lud.c
// Licence: /rodinia_3.1/LICENSE

use crate::utils::matrix::SquareMatrix;
use core::sync::atomic::Ordering;
use std::sync::atomic::AtomicU64;
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

// The workload of LU decomposition is parallelised as follows.
// Compared to the sequential implementation, we perform OUTER_BLOCK_SIZE of the outer loop at the same time.
// First we perform the LU decomposition of the OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE elements at the left top of the matrix.
// This happens in diagonal_tile, as is sequential.
// Then we propagate this information over the left border and the top border,
// i.e. the first OUTER_BLOCK_SIZE columns and OUTER_BLOCK_SIZE rows.
// This happens in task_border_go and is parallel.
// Then we work in the remaining part of the matrix, the interior. As soon as the work for the top left of the interior is completed,
// diagonal_tile is already executed for the next iteration.
// When all interior parts are handled, the next iteration contines (with the border and then the interior).
// This repeats until the entire matrix is handled.

// The outer loop is split in tiles of OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE elements.
const OUTER_BLOCK_SIZE: usize = 32;
// The left border and top border are handled in chunks of BORDER_BLOCK_SIZE elements.
const BORDER_BLOCK_SIZE: usize = 16;
// The inner part of the array is handled in tiles of the following size.
const INNER_BLOCK_SIZE_ROWS: usize = 16;
const INNER_BLOCK_SIZE_COLUMNS: usize = 32;

// The matrix size should be a multiple of OUTER_BLOCK_SIZE.
// OUTER_BLOCK_SIZE should be a multiple of BORDER_BLOCK_SIZE, INNER_BLOCK_SIZE_ROWS and INNER_BLOCK_SIZE_COLUMNS.

pub fn create_task(matrices: &[(SquareMatrix, AtomicU64)], pending: &AtomicU64) -> Task {
  pending.store(matrices.len() as u64, Ordering::Relaxed);
  Task::new_dataparallel::<Init>(task_init_go, task_init_finish, Init{ matrices, pending }, matrices.len() as u32)
}

struct Init<'a> {
  matrices: &'a[(SquareMatrix, AtomicU64)],
  pending: &'a AtomicU64
}

fn task_init_go(workers: &Workers, data: &Init, loop_arguments: LoopArguments) {
  workassisting_loop!(loop_arguments, |index| {
    let (matrix, synchronisation_var) = &data.matrices[index as usize];
    diagonal_tile(0, matrix);
    start_iteration(workers, 0, matrix, synchronisation_var, data.pending)
  });
}

fn task_init_finish(_workers: &Workers, _data: &Init) {
}

struct Data<'a> {
  matrix: &'a SquareMatrix,
  offset: usize,
  synchronisation_var: &'a AtomicU64,
  pending: &'a AtomicU64,
}

fn start_iteration(workers: &Workers, offset: usize, matrix: &SquareMatrix, synchronisation_var: &AtomicU64, pending: &AtomicU64) {
  let i_end = offset + OUTER_BLOCK_SIZE;

  if offset + OUTER_BLOCK_SIZE >= matrix.size() {
    // Work for this matrix is finished. Check if this was the last matrix.
    let old = pending.fetch_sub(1, Ordering::Relaxed);
    if old == 1 {
      workers.finish();
    }
  } else {
    // Continue with remaining part of the matrix
    let remaining = matrix.size() - i_end;

    synchronisation_var.store(0, Ordering::Relaxed);

    workers.push_task(
      Task::new_dataparallel::<Data>(
        task_border_left_go,
        task_border_finish,
        Data{ matrix, offset, synchronisation_var, pending },
        ((remaining + BORDER_BLOCK_SIZE - 1) / BORDER_BLOCK_SIZE) as u32
      )
    );
    workers.push_task(
      Task::new_dataparallel::<Data>(
        task_border_top_go,
        task_border_finish,
        Data{ matrix, offset, synchronisation_var, pending },
        ((remaining + BORDER_BLOCK_SIZE - 1) / BORDER_BLOCK_SIZE) as u32
      )
    );
  }
}

// Handles the tile on the diagonal, at the start of a new iteration.
fn diagonal_tile(offset: usize, matrix: &SquareMatrix) {
  for i in 0 .. OUTER_BLOCK_SIZE {
    for j in i .. OUTER_BLOCK_SIZE {
      for k in 0 .. i {
        matrix.write((offset + i, offset + j), matrix[(offset + i, offset + j)] - matrix[(offset + i, offset + k)] * matrix[(offset + k, offset + j)]);
      }
    }

    let temp = 1.0 / matrix[(offset + i, offset + i)];
    for j in i + 1 .. OUTER_BLOCK_SIZE {
      for k in 0 .. i {
        matrix.write((offset + j, offset + i), matrix[(offset + j, offset + i)] - matrix[(offset + j, offset + k)] * matrix[(offset + k, offset + i)]);
      }
      matrix.write((offset + j, offset + i), matrix[(offset + j, offset + i)] * temp);
    }
  }
}

fn task_border_left_go(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let mut temp = [0.0; OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE];

  for i in 0 .. OUTER_BLOCK_SIZE {
    for j in 0 .. OUTER_BLOCK_SIZE {
      temp[i * OUTER_BLOCK_SIZE + j] = data.matrix[(i + data.offset, j + data.offset)];
    }
  }

  workassisting_loop!(loop_arguments, |chunk_index| {
    let offset = data.offset;
    let matrix = data.matrix;

    let i_global = offset + OUTER_BLOCK_SIZE + BORDER_BLOCK_SIZE * (chunk_index as usize);
    let j_global = offset;
    for j in 0 .. OUTER_BLOCK_SIZE {
      for i in 0 .. BORDER_BLOCK_SIZE {
        let mut sum = 0.0;
        for k in 0 .. j {
          sum += matrix[(i_global + i, j_global + k)] * temp[OUTER_BLOCK_SIZE * k + j];
        }
        matrix.write(
          (i_global + i, j_global + j),
          (matrix[(i_global + i, j_global + j)] - sum) / matrix[(offset + j, offset + j)]
        );
      }
    }
  });
}

fn task_border_top_go(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let mut temp = [0.0; OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE];

  for i in 0 .. OUTER_BLOCK_SIZE {
    for j in 0 .. OUTER_BLOCK_SIZE {
      temp[i * OUTER_BLOCK_SIZE + j] = data.matrix[(i + data.offset, j + data.offset)];
    }
  }

  workassisting_loop!(loop_arguments, |chunk_index| {
    let offset = data.offset;
    let matrix = data.matrix;

    let i_global = offset;
    let j_global = offset + OUTER_BLOCK_SIZE + BORDER_BLOCK_SIZE * (chunk_index as usize);
    for j in 0 .. BORDER_BLOCK_SIZE {
      for i in 0 .. OUTER_BLOCK_SIZE {
        let mut sum = 0.0;
        for k in 0 .. i {
          sum += temp[i * OUTER_BLOCK_SIZE + k] * matrix[(i_global + k, j_global + j)];
        }
        matrix.write((i_global + i, j_global + j), matrix[(i_global + i, j_global + j)] - sum);
      }
    }
  });
}

fn task_border_finish(workers: &Workers, data: &Data) {
  // The algorithm continues when both the left and the top parts have finished.
  // This function handles the finish phase of both tasks.
  // The first task to finish sets the synchronisation var to 1 (the CAS succeeds).
  // The CAS fails in the second task, which signals that the algorithm can continue.
  let cas = data.synchronisation_var.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
  if cas.is_ok() { return; }

  let remaining = data.matrix.size() - data.offset - OUTER_BLOCK_SIZE;
  let rows = (remaining + INNER_BLOCK_SIZE_ROWS - 1) / INNER_BLOCK_SIZE_ROWS;
  let columns = (remaining + INNER_BLOCK_SIZE_COLUMNS - 1) / INNER_BLOCK_SIZE_COLUMNS;

  let inner_tiles_per_outer_tile = (OUTER_BLOCK_SIZE / INNER_BLOCK_SIZE_COLUMNS) * (OUTER_BLOCK_SIZE / INNER_BLOCK_SIZE_ROWS);
  data.synchronisation_var.store(inner_tiles_per_outer_tile as u64, Ordering::Relaxed);

  workers.push_task(
    Task::new_dataparallel::<Data>(
      task_inner_go,
      task_inner_finish,
      Data{ matrix: data.matrix, offset: data.offset, synchronisation_var: data.synchronisation_var, pending: data.pending },
      (rows * columns) as u32
    )
  );
}

fn task_inner_go(_workers: &Workers, data: &Data, loop_arguments: LoopArguments) {
  let remaining = data.matrix.size() - data.offset - OUTER_BLOCK_SIZE;
  let rows = (remaining + INNER_BLOCK_SIZE_ROWS - 1) / INNER_BLOCK_SIZE_ROWS;

  workassisting_loop!(loop_arguments, |chunk_index| {
    let mut temp_top = [0.0; INNER_BLOCK_SIZE_COLUMNS * OUTER_BLOCK_SIZE];
    let mut temp_left = [0.0; OUTER_BLOCK_SIZE * INNER_BLOCK_SIZE_ROWS];
    let mut sum = [0.0; max(INNER_BLOCK_SIZE_COLUMNS, INNER_BLOCK_SIZE_ROWS)];

    let i_global = data.offset + OUTER_BLOCK_SIZE + INNER_BLOCK_SIZE_ROWS * (chunk_index as usize % rows);
    let j_global = data.offset + OUTER_BLOCK_SIZE + INNER_BLOCK_SIZE_COLUMNS * (chunk_index as usize / rows);

    for i in 0 .. OUTER_BLOCK_SIZE {
      for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
        temp_top[i * INNER_BLOCK_SIZE_COLUMNS + j] = data.matrix[(i + data.offset, j + j_global)];
      }
    }

    for i in 0 .. INNER_BLOCK_SIZE_ROWS {
      for j in 0 .. OUTER_BLOCK_SIZE {
        temp_left[i * OUTER_BLOCK_SIZE + j] = data.matrix[(i + i_global, j + data.offset)];
      }
    }

    for i in 0 .. INNER_BLOCK_SIZE_ROWS {
      for k in 0 .. OUTER_BLOCK_SIZE {
        for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
          sum[j] += temp_left[OUTER_BLOCK_SIZE * i + k] * temp_top[INNER_BLOCK_SIZE_COLUMNS * k + j];
        }
      }
      for j in 0 .. INNER_BLOCK_SIZE_COLUMNS {
        data.matrix.write((i + i_global, j + j_global), data.matrix[(i + i_global, j + j_global)] - sum[j]);
        sum[j] = 0.0;
      }
    }

    if i_global < data.offset + 2 * OUTER_BLOCK_SIZE && j_global < data.offset + 2 * OUTER_BLOCK_SIZE {
      let old_remaining = data.synchronisation_var.fetch_sub(1, Ordering::AcqRel);
      if old_remaining == 1 {
        // All inner chunks of the first chunk (in terms of outer chunk sizes) are finished.
        // Start working on the diagonal chunk of the next iteration already.
        diagonal_tile(data.offset + OUTER_BLOCK_SIZE, data.matrix);
      }
    }
  });
}

fn task_inner_finish(workers: &Workers, data: &Data) {
  start_iteration(workers, data.offset + OUTER_BLOCK_SIZE, data.matrix, data.synchronisation_var, data.pending);
}

// https://stackoverflow.com/questions/53619695/calculating-maximum-value-of-a-set-of-constant-expressions-at-compile-time
const fn max(a: usize, b: usize) -> usize {
  [a, b][(a < b) as usize]
}
