# Work Assisting: Work Stealing for Data Parallelism

This repository contains an implementation of Work Assisting, an algorithm to schedule data parallel tasks. The implementation can be found in `src/core` and benchmarks in `src/cases`. The benchmarks compare work assisting with work stealing with a deque (`src/utils/deque_stealer.rs`) and Rayon, a framework for parallelism in Rust. For details about work assisting and the benchmarks we refer to the paper.

To run the benchmarks, the Rust compiler and cargo need to be installed. Furthermore gnuplot needs to be installed, as the benchmark code automatically generates charts of the results. The benchmarks can be run with `cargo run`. Depending on the processor, it may be needed to tune `AFFINITY_MAPPING` in `src/utils/thread_pinning.rs`. This specifies the order in which the cores of the processor are used.
