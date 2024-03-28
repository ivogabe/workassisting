#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <atomic>

void compact(uint64_t size, uint64_t* input, uint64_t* output) {
  int d = 0;
  #pragma omp parallel for reduction (inscan, +:d) schedule(dynamic,1)
  for (int j = 0; j < size; j += 4096) {
    int local_count = 0;
    for (int k = 0; k < 4096; k++) {
      if ((input[j + k] & 1) == 0) {
        output[d + local_count] += 1;
        local_count += 1;
      }
    }
    #pragma omp scan exclusive(d)
    int local_count2 = 0;
    for (int k = 0; k < 4096; k++) {
      if ((input[j + k] & 1) == 0) {
        local_count2 += 1;
      }
    }
    d += local_count2;
  }
}

void case_compact(int thread_count, uint64_t array_count, uint64_t size, uint64_t** inputs, uint64_t** outputs) {
  if (array_count == 1) {
    compact(size, inputs[0], outputs[0]);
    return;
  }

  omp_set_max_active_levels(2);
  omp_set_num_threads(array_count);

  #pragma omp parallel for
  for (int i = 0; i < array_count; i++) {
    uint64_t* input = inputs[i];
    uint64_t* output = outputs[i];

    omp_set_num_threads(thread_count);
    compact(size, inputs[i], outputs[i]);
  }
}

uint32_t compact_random(uint64_t seed) {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  return (uint32_t) seed;
}

uint64_t** compact_alloc(uint64_t array_count, uint64_t size, bool fill) {
  uint64_t** result = new uint64_t*[array_count];
  for (int i = 0; i < array_count; i++) {
    result[i] = new uint64_t[size];
    if (fill) {
      for (int j = 0; j < size; j++) {
        result[i][j] = compact_random(j);
      }
    }
  }
  return result;
}
