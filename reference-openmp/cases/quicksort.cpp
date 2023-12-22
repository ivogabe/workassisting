#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <atomic>

#define BLOCK_SIZE 512
#define DATAPAR_CUTOFF 32768
#define SEQUENTIAL_CUTOFF 8192
#define INSERTION_SORT_CUTOFF 20

uint32_t random(uint64_t seed) {
  seed += 876998787696;
  seed *= 35334534876231;
  seed ^= seed << 19;
  seed ^= seed >> 23;
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  return (uint32_t) (seed & 0xFFFFFFFF);
}

void fill(uint32_t* array, int size) {
  #pragma omp parallel for schedule(static,8192)
  for (int i = 0; i < size; i++) {
    array[i] = random(i);
  }
}

void parallel_partition_block(uint32_t* input, uint32_t* output, int size, uint32_t pivot, std::atomic<uint64_t>* counters, int block_index) {
  // Loop starts at 1, as element 0 is the pivot.
  int start = 1 + block_index * BLOCK_SIZE ;
  int end = start + BLOCK_SIZE;
  if (end > size) {
    end = size;
  }

  uint32_t values[BLOCK_SIZE];
  int left_count = 0;
  int right_count = 0;
  for (int i = 0; i < end - start; i++) {
    uint32_t value = input[start + i];
    int destination;
    if (value < pivot) {
      destination = left_count;
      left_count += 1;
    } else {
      destination = right_count;
      right_count += 1;
    }
    values[destination] = value;
  }

  uint64_t counters_value = std::atomic_fetch_add_explicit(counters, ((uint64_t) right_count << 32) | left_count, std::memory_order_seq_cst);
  int left_offset = counters_value & 0xFFFFFFFF;
  int right_offset = size - right_count - (counters_value >> 32);

  for (int i = 0; i < left_count; i++) {
    output[left_offset + i] = values[i];
  }
  for (int i = 0; i < right_count; i++) {
    output[right_offset + i] = values[i];
  }
}

int parallel_partition(uint32_t* input, uint32_t* output, int size) {
  int block_count = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;

  uint32_t pivot = input[0];

  std::atomic<uint64_t> counters;
  std::atomic_store_explicit(&counters, 0, std::memory_order_relaxed);

  #pragma omp parallel for schedule(dynamic,1)
  for (int block_index = 0; block_index < block_count; block_index++) {
    parallel_partition_block(input, output, size, pivot, &counters, block_index);
  }

  uint64_t counters_value = std::atomic_load_explicit(&counters, std::memory_order_relaxed);
  int count_left = counters & 0xFFFFFFFF;
  int count_right = counters >> 32;
  if (count_left + count_right + 1 != size) {
    printf("Size mismatch\n");
  }

  // Return the position of the pivot
  return count_left;
}

int parallel_partition_taskloop(uint32_t* input, uint32_t* output, int size) {
  int block_count = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;

  uint32_t pivot = input[0];

  std::atomic<uint64_t> counters;
  std::atomic_store_explicit(&counters, 0, std::memory_order_relaxed);
  std::atomic<uint64_t>* counters_ptr = &counters;

  #pragma omp taskloop grainsize(1)
  for (int block_index = 0; block_index < block_count; block_index++) {
    parallel_partition_block(input, output, size, pivot, counters_ptr, block_index);
  }

  uint64_t counters_value = std::atomic_load_explicit(&counters, std::memory_order_relaxed);
  int count_left = counters & 0xFFFFFFFF;
  int count_right = counters >> 32;
  if (count_left + count_right + 1 != size) {
    printf("Size mismatch\n");
  }

  // Return the position of the pivot
  return count_left;
}

int sequential_partition(uint32_t* array, int size) {
  uint32_t pivot = array[0];
  int left = 1;
  int right = size - 1;
  while (true) {
    while (left < size && array[left] < pivot) left++;
    while (right > 0 && array[right] >= pivot) right--;

    if (left >= right) break;
    uint32_t left_value = array[left];
    array[left] = array[right];
    array[right] = left_value;
    left++;
    right--;
  }
  if (left - 1 != right) {
    printf("Left and right indices incorrect\n");
  }
  return right;
}

void insertion_sort(uint32_t* array, int size) {
  for (int idx = 1; idx < size; idx++) {
    uint32_t value = array[idx];
    // Find position for this element.
    int j = idx;
    while (j > 0) {
      j -= 1;
      uint32_t current = array[j];
      if (current <= value) {
        j += 1;
        break;
      }
      array[j + 1] = current;
    }
    array[j] = value;
  }
}

void sequential_quicksort(uint32_t* array, int size) {
  if (size <= 1) {
    return;
  }
  if (size <= INSERTION_SORT_CUTOFF) {
    insertion_sort(array, size);
    return;
  }
  int pivot_idx = sequential_partition(array, size);

  // Place pivot at correct index, by swapping it with index 0
  uint32_t pivot = array[0];
  array[0] = array[pivot_idx];
  array[pivot_idx] = pivot;

  // Recursion
  sequential_quicksort(array, pivot_idx);
  sequential_quicksort(&array[pivot_idx + 1], size - pivot_idx - 1);
}

// Only task parallel. Data parallelism in partitioning is not exploited.
void only_task_par_quicksort(uint32_t* array, int size) {
  if (size < SEQUENTIAL_CUTOFF) {
    sequential_quicksort(array, size);
    return;
  }
  int pivot_idx = sequential_partition(array, size);

  // Place pivot at correct index, by swapping it with index 0
  uint32_t pivot = array[0];
  array[0] = array[pivot_idx];
  array[pivot_idx] = pivot;

  // Recursion
  #pragma omp task
  only_task_par_quicksort(array, pivot_idx);

  #pragma omp task
  only_task_par_quicksort(&array[pivot_idx + 1], size - pivot_idx - 1);

  #pragma omp taskwait
}

void quicksort(uint32_t* input, uint32_t* output, bool input_output_flipped, int size) {
  if (size < DATAPAR_CUTOFF) {
    if (!input_output_flipped) {
      // Copy input to output, then sort output
      for (int i = 0; i < size; i++) {
        output[i] = input[i];
      }
      only_task_par_quicksort(output, size);
    } else {
      only_task_par_quicksort(input, size);
    }
    return;
  }

  int pivot_idx = parallel_partition(input, output, size);

  // Store pivot at correct index. Since input and output constantly change roles, make sure that we write to the correct array.
  if (input_output_flipped) {
    input[pivot_idx] = input[0];
  } else {
    output[pivot_idx] = input[0];
  }

  // Recursion
  #pragma omp task
  quicksort(output, input, !input_output_flipped, pivot_idx);

  #pragma omp task
  quicksort(&output[pivot_idx + 1], &input[pivot_idx + 1], !input_output_flipped, size - pivot_idx - 1);
}

void quicksort_taskloop(uint32_t* input, uint32_t* output, bool input_output_flipped, int size) {
  if (size < DATAPAR_CUTOFF) {
    if (!input_output_flipped) {
      // Copy input to output, then sort output
      for (int i = 0; i < size; i++) {
        output[i] = input[i];
      }
      only_task_par_quicksort(output, size);
    } else {
      only_task_par_quicksort(input, size);
    }
    return;
  }

  int pivot_idx = parallel_partition_taskloop(input, output, size);

  // Store pivot at correct index. Since input and output constantly change roles, make sure that we write to the correct array.
  if (input_output_flipped) {
    input[pivot_idx] = input[0];
  } else {
    output[pivot_idx] = input[0];
  }

  // Recursion
  #pragma omp task
  quicksort_taskloop(output, input, !input_output_flipped, pivot_idx);

  #pragma omp task
  quicksort_taskloop(&output[pivot_idx + 1], &input[pivot_idx + 1], !input_output_flipped, size - pivot_idx - 1);
}

void case_quicksort(uint32_t* input, uint32_t* output, int size) {
  fill(input, size);
  #pragma omp parallel
  #pragma omp single
  {
    quicksort(input, output, false, size);
    #pragma omp taskwait
  }
}

void case_quicksort_taskloop(uint32_t* input, uint32_t* output, int size) {
  fill(input, size);
  #pragma omp parallel
  #pragma omp single
  {
    quicksort_taskloop(input, output, false, size);
    #pragma omp taskwait
  }
}
