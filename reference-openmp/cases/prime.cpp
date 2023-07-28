#include <omp.h>
#include <cstdio>
#include <cstdint>
#include <atomic>

// Primes benchmark
bool is_prime(uint64_t input) {
  if (input % 2 == 0 && input != 2) {
    return false;
  }

  uint64_t factor = 3;
  while (factor * factor <= input) {
    if (input % factor == 0) {
      return false;
    }
    factor += 2;
  }
  return true;
}

uint32_t case_primes_dynamic(uint64_t from, uint64_t to) {
  std::atomic_uint count(0);
  #pragma omp parallel
  {
    uint32_t local_count = 0;
    #pragma omp for schedule(dynamic,32)
    for (uint64_t i = from; i < to; i++) {
      if (is_prime(i)) {
        local_count++;
      }
    }
    count += local_count;
  }
  return (uint32_t) count;
}

uint32_t case_primes_static(uint64_t from, uint64_t to) {
  std::atomic_uint count(0);
  #pragma omp parallel
  {
    uint32_t local_count = 0;
    #pragma omp for schedule(static,32)
    for (uint64_t i = from; i < to; i++) {
      if (is_prime(i)) {
        local_count++;
      }
    }
    count += local_count;
  }
  return (uint32_t) count;
}
