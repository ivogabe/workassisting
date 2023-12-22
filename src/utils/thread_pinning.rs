// For Intel 12900:
// First use performance cores without hyperthreading,
// then efficiency cores,
// then hyperthreaded performance cores.
pub const AFFINITY_MAPPING: [usize; 24] = [0, 2, 4, 6, 8, 10, 12, 14, 16, 17, 18, 19, 20, 21, 22, 23, 1, 3, 5, 7, 9, 11, 13, 15];
// For AMD 2950X:
// pub const AFFINITY_MAPPING: [usize; 32] = [0, 2, 4, 6, 8, 10, 12, 14, 1, 3, 5, 7, 9, 11, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31];
