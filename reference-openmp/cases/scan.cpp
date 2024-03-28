#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <atomic>

void scan(uint64_t size, uint64_t* input, uint64_t* output) {
  int d = 0;

  // This crashes in clang, as this will allocate a uint64_t[size] on the stack
  // and on smaller sizes it is really slow, hence we have an optimized version below.
  /*
  #pragma omp parallel for reduction (inscan, +:d) schedule(dynamic,14096)
  for (int j = 0; j < size; j += 1) {
    d += output[j];
    #pragma omp scan inclusive(d)
    output[j] = d;
  } */

  #pragma omp parallel for reduction (inscan, +:d) schedule(dynamic,1)
  for (int j = 0; j < size; j += 4096) {
    int prefix = d;
    for (int k = 0; k < 4096; k++) {
      prefix += input[j + k];
      output[j + k] = prefix;
    }
    #pragma omp scan inclusive(d)
    int accum = 0;
    for (int k = 0; k < 4096; k++) {
      accum += input[j + k];
    }
    d += accum;
  }
}

void case_scan(uint64_t array_count, uint64_t size, uint64_t** inputs, uint64_t** outputs) {
  if (array_count == 1) {
    scan(size, inputs[0], outputs[0]);
    return;
  }

  #pragma omp parallel for
  for (int i = 0; i < array_count; i++) {
    uint64_t* input = inputs[i];
    uint64_t* output = outputs[i];

    scan(size, inputs[i], outputs[i]);
  }
}

uint32_t scan_random(uint64_t seed) {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  return (uint32_t) seed;
}

uint64_t** scan_alloc(uint64_t array_count, uint64_t size, bool fill) {
  uint64_t** result = new uint64_t*[array_count];
  for (int i = 0; i < array_count; i++) {
    result[i] = new uint64_t[size];
    if (fill) {
      for (int j = 0; j < size; j++) {
        result[i][j] = scan_random(j);
      }
    }
  }
  return result;
}
