#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <atomic>

uint32_t randomize(uint64_t seed) {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  return (uint32_t) seed;
}

uint64_t* sum_create_input(int size) {
  uint64_t* array = new uint64_t[size];
  for (int i = 0; i < size; i++) {
    array[i] = randomize(i);
  }
  return array;
}

uint64_t case_sum_array_dynamic(uint64_t* array, int size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(dynamic,8192)
  for (int i=0; i<size; i++) {
    sum = sum + array[i];
  }
  return sum;
}

uint64_t case_sum_array_static(uint64_t* array, int size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(static,8192)
  for (int i=0; i<size; i++) {
    sum = sum + array[i];
  }
  return sum;
}

uint64_t case_sum_function_dynamic(int size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(dynamic,4096)
  for (int i=0; i<size; i++) {
    sum = sum + randomize(i);
  }
  return sum;
}

uint64_t case_sum_function_static(int size) {
  uint64_t sum = 0;
  #pragma omp parallel for reduction(+:sum) schedule(static,4096)
  for (int i=0; i<size; i++) {
    sum = sum + randomize(i);
  }
  return sum;
}
