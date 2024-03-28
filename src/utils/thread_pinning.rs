use std::io::Write;

use super::benchmark::time;

// Specifies the order in which threads should be used. Tests with P threads use cores AFFINITY_MAPPING[0 .. P].
// For AMD 2950X:
pub const AFFINITY_MAPPING: [usize; 32] = [28, 15, 22, 17, 20, 9, 18, 11, 29, 21, 30, 26, 27, 31, 5, 2, 25, 13, 14, 10, 8, 1, 16, 6, 0, 24, 4, 12, 23, 3, 7, 19];

pub fn find_best_affinity_mapping<F: FnMut(&[usize])>(cores: usize, mut f: F) -> Box<[usize]> {
  println!("Finding best thread affinities. This may take a while.");

  let mut result = vec![];
  for i in 0 .. cores {
    let mut best_time = u64::MAX;
    let mut best_core = 0;
    for core in 0 .. cores {
      if result.contains(&core) {
        continue;
      }
      result.push(core);
      let (_, t) = time(10, || f(&result));
      if t < best_time {
        best_time = t;
        best_core = core;
      }
      result.pop();
    }
    if i == 0 {
      print!("  [{}", best_core);
    } else {
      print!(", {}", best_core);
    }
    std::io::stdout().flush().unwrap();
    result.push(best_core);
  }
  println!("]");
  println!();
  println!("Modify the definition of AFFINITY_MAPPING in src/utils/thread_pinning.rs to:");
  println!("pub const AFFINITY_MAPPING: [usize; {}] = {:?};", cores, result);
  result.into_boxed_slice()
}
