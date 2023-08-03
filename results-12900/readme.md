This folder contains the results of the benchmarks executed on an Intel 12900. The thread pinning was configured to first utilise all performance cores, without hyperthreading, and if that is not sufficient, then efficiency cores, and if that is still not sufficient, then hyperthreaded performance cores. This can be achieved by changing the contents of `utils/thread_pinning.rs` to:

```rust
pub const AFFINITY_MAPPING: [usize; 24] = [0, 2, 4, 6, 8, 10, 12, 14, 16, 17, 18, 19, 20, 21, 22, 23, 1, 3, 5, 7, 9, 11, 13, 15];
```
