# Work Assisting: Linking Task-Parallel Work Stealing with Loop-Parallel Self Scheduling

This repository contains an implementation of *work assisting*, a scheduling strategy for mixing loop parallelism (data parallelism) with task parallelism.
Threads share their current loop-parallel activity in a shared array to let other threads assist.
Our algorithm aims at preserving the structure of loop parallelism instead of implementing all parallelism as task
parallelism. This avoids the scheduling overhead introduced
by the latter and enables the use of self-scheduling for loop parallelism.
It provides full flexibility: Neither the number of threads for a parallel loop nor the distribution over
threads need to be fixed before the loop starts. We present benchmarks to demonstrate that our scheduling algorithm behaves
similar to or outperforms schedulers based purely on task parallelism.

The implementation can be found in `src/core` and benchmarks in `src/cases`. The benchmarks compare work assisting with work stealing with a deque (`src/utils/deque_stealer.rs`) and Rayon, a framework for parallelism in Rust. For details about work assisting and the benchmarks we refer to the paper.

## Instructions
To run the benchmarks, the Rust compiler and cargo need to be installed. Furthermore gnuplot needs to be installed, as the benchmark code automatically generates charts of the results. The benchmarks can be run with `cargo run`. Depending on the processor, it may be needed to tune `AFFINITY_MAPPING` in `src/utils/thread_pinning.rs`. This specifies the order in which the cores of the processor are used.

The program will automatically try to build OpenMP-based implementations. This requires that gcc is installed with OpenMP. If the compilation of the OpenMP code fails, the program will proceed with running the benchmarks without OpenMP.
