#include <omp.h>
#include <cstdint>
#include <cstdio>
#include <chrono>
#include <cstring>
#include <string>

#define RUNS 20

uint32_t case_primes_dynamic(uint64_t, uint64_t);
uint32_t case_primes_static(uint64_t, uint64_t);

uint64_t* sum_create_input(int size);
uint64_t case_sum_array_dynamic(uint64_t*, int);
uint64_t case_sum_array_static(uint64_t*, int);
uint64_t case_sum_function_dynamic(int);
uint64_t case_sum_function_static(int);

void case_quicksort(uint32_t*, uint32_t*, int);

int main(int argc, char *argv[]) {
  if (argc < 2) {
    printf("Usage: ./main test-case ...\n");
    return 0;
  }

  // Switch on test case
  if (std::strcmp(argv[1], "prime-dynamic") == 0) {
    if (argc < 4) {
      printf("Usage: ./main prime-dynamic lowerbound upperbound\n");
      return 0;
    }

    int lower = std::stoi(argv[2]);
    int upper = std::stoi(argv[3]);

    // Warm-up run
    case_primes_dynamic(lower, upper);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_primes_dynamic(lower, upper);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else if (std::strcmp(argv[1], "prime-static") == 0) {
    if (argc < 4) {
      printf("Usage: ./main prime-static lowerbound upperbound\n");
      return 0;
    }

    int lower = std::stoi(argv[2]);
    int upper = std::stoi(argv[3]);

    // Warm-up run
    case_primes_static(lower, upper);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_primes_static(lower, upper);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else if (std::strcmp(argv[1], "sum-array-dynamic") == 0) {
    if (argc < 3) {
      printf("Usage: ./main sum-array-dynamic size\n");
      return 0;
    }

    int size = std::stoi(argv[2]);

    uint64_t* array = sum_create_input(size);

    // Warm-up run
    case_sum_array_dynamic(array, size);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_sum_array_dynamic(array, size);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

} else if (std::strcmp(argv[1], "sum-array-static") == 0) {
    if (argc < 3) {
      printf("Usage: ./main sum-array-static size\n");
      return 0;
    }

    int size = std::stoi(argv[2]);

    uint64_t* array = sum_create_input(size);

    // Warm-up run
    case_sum_array_static(array, size);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_sum_array_static(array, size);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

} else if (std::strcmp(argv[1], "sum-function-dynamic") == 0) {
    if (argc < 3) {
      printf("Usage: ./main sum-function-dynamic size\n");
      return 0;
    }

    int size = std::stoi(argv[2]);

    // Warm-up run
    case_sum_function_dynamic(size);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_sum_function_dynamic(size);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

} else if (std::strcmp(argv[1], "sum-function-static") == 0) {
    if (argc < 3) {
      printf("Usage: ./main sum-function-static size\n");
      return 0;
    }

    int size = std::stoi(argv[2]);

    // Warm-up run
    case_sum_function_static(size);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_sum_function_static(size);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

} else if (std::strcmp(argv[1], "quicksort") == 0) {
    if (argc < 3) {
      printf("Usage: ./main quicksort size\n");
      return 0;
    }

    int size = std::stoi(argv[2]);

    uint32_t* input = new uint32_t[size];
    uint32_t* output = new uint32_t[size];

    // Warm-up run
    case_quicksort(input, output, size);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      case_quicksort(input, output, size);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else {
    printf("Unknown test case.");
  }

  return 0;
}
