#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <cinttypes>
#include <atomic>
#include <cmath>

uint32_t randomize(uint64_t seed) {
  // This function has equal performance in rust and clang
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed = (uint64_t) (cos((double) seed) * 123456789.0);
  seed ^= seed << 5;
  seed *= seed;
  seed += 9023;
  seed ^= seed >> 11;
  seed ^= seed << 9;
  return (uint32_t) seed;
}

uint64_t* sum_create_input(uint64_t size) {
  uint64_t* array = new uint64_t[size];
  for (uint64_t i = 0; i < size; i++) {
    array[i] = randomize(i);
  }
  return array;
}

uint64_t case_sum_array_dynamic(uint64_t* array, uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(dynamic,8192)
  for (uint64_t i=0; i<size; i++) {
    sum = sum + array[i];
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}

uint64_t case_sum_array_static(uint64_t* array, uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(static,8192)
  for (uint64_t i=0; i<size; i++) {
    sum = sum + array[i];
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}

uint64_t case_sum_array_taskloop(uint64_t* array, uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel
  #pragma omp single
  {
    #pragma omp taskloop reduction(+:sum) grainsize(8192)
    for (uint64_t i=0; i<size; i++) {
      sum = sum + array[i];
    }
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}

uint64_t case_sum_function_dynamic(uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(dynamic,512)
  for (uint64_t i=0; i<size; i++) {
    sum = sum + randomize(i);
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}

uint64_t case_sum_function_static(uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(static,512)
  for (uint64_t i=0; i<size; i++) {
    sum = sum + randomize(i);
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}

uint64_t case_sum_function_taskloop(uint64_t size) {
  uint64_t sum = 0;
  #pragma omp parallel
  #pragma omp single
  {
    #pragma omp taskloop reduction(+:sum) grainsize(512)
    for (uint64_t i=0; i<size; i++) {
      sum = sum + randomize(i);
    }
  }
  fprintf(stderr, "%" PRIu64 "\n", sum);
  return sum;
}
