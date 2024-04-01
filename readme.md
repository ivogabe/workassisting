# Work Assisting: Linking Task-Parallel Work Stealing with Loop-Parallel Self Scheduling

This repository contains an implementation of *work assisting*, a scheduling strategy for mixing data parallelism (loop parallelism) with task parallelism.
Threads share their current data-parallel activity in a shared array to let other threads assist.
In contrast to most existing work in this space, our algorithm aims at preserving the structure of data parallelism instead of implementing all parallelism as task  parallelism.
This enables the use of self-scheduling for data parallelism, as required by certain data-parallel algorithms,
  and only exploits data parallelism if task parallelism is not sufficient.
It provides full flexibility: neither the number of threads for a data-parallel loop nor the distribution over
threads need to be fixed before the loop starts. We present benchmarks to demonstrate that our scheduling algorithm, depending on the problem, behaves
similar to, or outperforms schedulers based purely on task parallelism.

The implementation can be found in `src/core` and benchmarks in `src/cases`. The benchmarks compare work assisting with work stealing with a deque (`src/utils/deque_stealer.rs`) and Rayon, a framework for parallelism in Rust. For details about work assisting and the benchmarks we refer to the paper.

## Instructions
To run the benchmarks, the Rust compiler and cargo need to be installed. Furthermore gnuplot needs to be installed, as the benchmark code automatically generates charts of the results.

The benchmarks set thread affinities to run the algorithms on a specific number of cores. The order in which cores are used needs to be tuned, especially when the processor has a NUMA architecture or has performance and efficiency cores. By running `cargo run -- affinities`, this program will find a proper configuration. Afterwards, that configuration should be specified in `AFFINITY_MAPPING` in `src/utils/thread_pinning.rs`. 

The benchmarks can be run with `cargo run`.

The program will automatically try to build OpenMP-based implementations. This requires that gcc is installed with OpenMP. If the compilation of the OpenMP code fails, the program will proceed with running the benchmarks without OpenMP.
