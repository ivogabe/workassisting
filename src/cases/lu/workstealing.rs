// Implementation of LU decomposition.
// The algorithm follows the structure of the implementation with our work assisting scheduler (./our.rs),
// but uses work-stealing for scheduling.
// This implementation is mainly based on the OpenMP implementation from Rodinia.
// Source: /rodinia_3.1/openmp/lud/omp/lud.c
// Licence: /rodinia_3.1/LICENSE

use core::sync::atomic::Ordering;
use std::sync::atomic::AtomicU64;
use crate::utils::matrix::SquareMatrix;
use crate::utils::deque_stealer::*;
use super::*;

// The outer loop is split in tiles of OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE elements.
const OUTER_BLOCK_SIZE: usize = 32;
// The left border and top border are handled in chunks of BORDER_BLOCK_SIZE elements.
const BORDER_BLOCK_SIZE: usize = 32;
// The inner part of the array is handled in tiles of the following size.
const INNER_BLOCK_SIZE_ROWS: usize = 16;
const INNER_BLOCK_SIZE_COLUMNS: usize = 32;

pub fn run(matrices: &[(SquareMatrix, AtomicU64, AtomicU64)], pending: &AtomicU64, thread_count: usize) {
  pending.store(matrices.len() as u64, Ordering::Relaxed);

  let tasks: Vec<Task> = matrices.iter().map(|(matrix, synchronisation_var1, synchronisation_var2)|
    Task::new(
      task_start,
      Box::new(Start{ matrix, synchronisation_var1, synchronisation_var2, pending })
    )
  ).collect();

  run_with_workstealing(tasks, thread_count);
}

struct Start<'a> {
  matrix: &'a SquareMatrix,
  synchronisation_var1: &'a AtomicU64,
  synchronisation_var2: &'a AtomicU64,
  pending: &'a AtomicU64,
}

// Performs the first iteration of LU decomposition and schedule the later work.
fn task_start(worker: Worker, data: Box<Start>) {
  diagonal_tile(0, data.matrix);
  schedule_border(worker, data.matrix, 0, data.synchronisation_var1, data.synchronisation_var2, data.pending);
}

// Schedule tasks for the left and top border
fn schedule_border(
  worker: Worker,
  matrix: &SquareMatrix,
  offset: usize,
  synchronisation_var1: &AtomicU64,
  synchronisation_var2: &AtomicU64,
  pending: &AtomicU64
) {
  let tasks = (matrix.size() - offset - OUTER_BLOCK_SIZE) / BORDER_BLOCK_SIZE;

  synchronisation_var1.store(tasks as u64 * 2, Ordering::Relaxed);

  for chunk_index in 0 .. tasks {
    worker.push_task(Task::new(
      task_border_left,
      Box::new(Border { matrix, offset, synchronisation_var1, synchronisation_var2, pending, chunk_index })
    ));
    worker.push_task(Task::new(
      task_border_top,
      Box::new(Border { matrix, offset, synchronisation_var1, synchronisation_var2, pending, chunk_index })
    ));
  }
}

struct Border<'a> {
  matrix: &'a SquareMatrix,
  offset: usize,
  synchronisation_var1: &'a AtomicU64,
  synchronisation_var2: &'a AtomicU64,
  pending: &'a AtomicU64,
  chunk_index: usize
}

fn task_border_left(worker: Worker, data: Box<Border>) {
  let mut temp = Align([0.0; OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE]);

  border_init(data.offset, data.matrix, &mut temp);
  border_left_chunk::<BORDER_BLOCK_SIZE>(data.offset, data.matrix, &temp, data.chunk_index);

  if data.synchronisation_var1.fetch_sub(1, Ordering::AcqRel) == 1 {
    schedule_interior(worker, data.matrix, data.offset, data.synchronisation_var1, data.synchronisation_var2, data.pending);
  }
}

fn task_border_top(worker: Worker, data: Box<Border>) {
  let mut temp = Align([0.0; OUTER_BLOCK_SIZE * OUTER_BLOCK_SIZE]);

  border_init(data.offset, data.matrix, &mut temp);
  border_top_chunk::<BORDER_BLOCK_SIZE>(data.offset, data.matrix, &temp, data.chunk_index);

  if data.synchronisation_var1.fetch_sub(1, Ordering::AcqRel) == 1 {
    schedule_interior(worker, data.matrix, data.offset, data.synchronisation_var1, data.synchronisation_var2, data.pending);
  }
}

fn schedule_interior(
  worker: Worker,
  matrix: &SquareMatrix,
  offset: usize,
  synchronisation_var1: &AtomicU64,
  synchronisation_var2: &AtomicU64,
  pending: &AtomicU64
) {
  let remaining = matrix.size() - offset - OUTER_BLOCK_SIZE;
  let rows = (remaining + INNER_BLOCK_SIZE_ROWS - 1) / INNER_BLOCK_SIZE_ROWS;
  let columns = (remaining + INNER_BLOCK_SIZE_COLUMNS - 1) / INNER_BLOCK_SIZE_COLUMNS;

  let chunks = rows * columns;

  let inner_tiles_per_outer_tile = (OUTER_BLOCK_SIZE / INNER_BLOCK_SIZE_COLUMNS) * (OUTER_BLOCK_SIZE / INNER_BLOCK_SIZE_ROWS);
  synchronisation_var1.store(inner_tiles_per_outer_tile as u64, Ordering::Relaxed);
  synchronisation_var2.store(chunks as u64, Ordering::Relaxed);

  // Split workload in some tasks. It will be split in more tasks later on, recursively.
  for i in 0 .. 4 {
    let chunk_start = chunks * i / 4;
    let chunk_end = chunks * (i + 1) / 4;
    if chunk_start == chunk_end { continue; }

    worker.push_task(Task::new(
      task_interior,
      Box::new(Interior { matrix, offset, synchronisation_var1, synchronisation_var2, pending, rows, chunk_start, chunk_end })
    ));
  }
}

fn task_interior(worker: Worker, data: Box<Interior>) {
  // Split range of this task in multiple tasks.
  // First is handled by this thread, others should be pushed to the queue.
  let chunk_sub_count = data.chunk_end - data.chunk_start - 1;
  for i in 0 .. 2 {
    let sub_start = chunk_sub_count * i / 2;
    let sub_end = chunk_sub_count * (i + 1) / 2;
    if sub_start == sub_end { continue; }

    worker.push_task(Task::new(
      task_interior,
      Box::new(Interior {
        matrix: data.matrix,
        offset: data.offset,
        synchronisation_var1: data.synchronisation_var1,
        synchronisation_var2: data.synchronisation_var2,
        pending: data.pending,
        rows: data.rows,
        chunk_start: data.chunk_start + 1 + sub_start,
        chunk_end: data.chunk_start + 1 + sub_end
      })
    ));
  }

  // Do work for first chunk of this range
  let chunk_index = data.chunk_start;
  let mut temp_top = Align([0.0; INNER_BLOCK_SIZE_COLUMNS * OUTER_BLOCK_SIZE]);
  let mut sum = Align([0.0; max(INNER_BLOCK_SIZE_COLUMNS, INNER_BLOCK_SIZE_ROWS)]);
  let mut temp_index = 0;
  
  interior_chunk::<INNER_BLOCK_SIZE_ROWS, INNER_BLOCK_SIZE_COLUMNS>
    (data.offset, data.rows, data.matrix, &mut temp_index, &mut temp_top.0, &mut sum.0, chunk_index);

  let i_global = data.offset + OUTER_BLOCK_SIZE + INNER_BLOCK_SIZE_ROWS * (chunk_index as usize % data.rows);
  let j_global = data.offset + OUTER_BLOCK_SIZE + INNER_BLOCK_SIZE_COLUMNS * (chunk_index as usize / data.rows);

  if i_global < data.offset + 2 * OUTER_BLOCK_SIZE && j_global < data.offset + 2 * OUTER_BLOCK_SIZE {
    let old_remaining = data.synchronisation_var1.fetch_sub(1, Ordering::AcqRel);
    if old_remaining == 1 {
      // All inner chunks of the first chunk (in terms of outer chunk sizes) are finished.
      // Start working on the diagonal chunk of the next iteration already.
      diagonal_tile(data.offset + OUTER_BLOCK_SIZE, data.matrix);
    }
  }

  if data.synchronisation_var2.fetch_sub(1, Ordering::AcqRel) == 1 {
    let offset = data.offset + OUTER_BLOCK_SIZE;
    // Start next iteration
    if offset + OUTER_BLOCK_SIZE >= data.matrix.size() {
      // Work for this matrix is finished. Check if this was the last matrix.
      let old = data.pending.fetch_sub(1, Ordering::Relaxed);
      if old == 1 {
        worker.finish();
      }
    } else {
      schedule_border(worker, data.matrix, offset, data.synchronisation_var1, data.synchronisation_var2, data.pending);
    }
  }
}

struct Interior<'a> {
  matrix: &'a SquareMatrix,
  offset: usize,
  synchronisation_var1: &'a AtomicU64,
  synchronisation_var2: &'a AtomicU64,
  pending: &'a AtomicU64,
  rows: usize,
  chunk_start: usize, // Inclusive
  chunk_end: usize // Exclusive
}

// https://stackoverflow.com/questions/53619695/calculating-maximum-value-of-a-set-of-constant-expressions-at-compile-time
const fn max(a: usize, b: usize) -> usize {
  [a, b][(a < b) as usize]
}
