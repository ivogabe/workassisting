mod core;
mod cases;
mod utils;

use std::path::Path;

use utils::thread_pinning::AFFINITY_MAPPING;

fn main() {
  let open_mp_enabled = build_open_mp();
  if !open_mp_enabled {
    println!("Running the benchmarks without the OpenMP implementations");
  }

  let full = affinity::get_thread_affinity().unwrap();
  affinity::set_thread_affinity([AFFINITY_MAPPING[0]]).unwrap();
  cases::lud::run(open_mp_enabled);
  affinity::set_thread_affinity(full).unwrap();
  cases::quicksort::run(open_mp_enabled);
  cases::prime::run(open_mp_enabled);
  cases::sum_array::run(open_mp_enabled);
  cases::sum_function::run(open_mp_enabled);
}

fn build_open_mp() -> bool {
  if !cfg!(unix) {
    return false;
  }

  println!("Building the OpenMP implementation of sum, primes and quicksort");
  match std::process::Command::new("sh").arg("./reference-openmp/build.sh").spawn() {
    Ok(mut child) => {
      match child.wait() {
        Ok(result) => {
          if !result.success() {
            println!("Build of OpenMP code failed.");
            return false;
          }
        }
        Err(_) => {
          println!("Build of OpenMP code failed.");
          return false;
        },
      }
    },
    Err(_) => {
      println!("Build of OpenMP code failed.");
      return false;
    }
  }

  println!("Building the OpenMP implementation of LU decomposition");
  let openmp_lu_path = Path::new("./rodinia_3.1/openmp/lud").canonicalize().unwrap();
  match std::process::Command::new("make").arg("lud_omp").current_dir(&openmp_lu_path).spawn() {
    Ok(mut child) => {
      match child.wait() {
        Ok(result) => {
          if !result.success() {
            println!("Build of OpenMP code failed.");
            return false;
          }
        }
        Err(_) => {
          println!("Build of OpenMP code failed.");
          return false;
        },
      }
    },
    Err(_) => {
      println!("Build of OpenMP code failed.");
      return false;
    }
  }

  let openmp_lu_taskloops_path = Path::new("./rodinia_3.1/openmp-taskloops/lud").canonicalize().unwrap();
  match std::process::Command::new("make").arg("lud_omp").current_dir(&openmp_lu_taskloops_path).spawn() {
    Ok(mut child) => {
      match child.wait() {
        Ok(result) => {
          if !result.success() {
            println!("Build of OpenMP code failed.");
            return false;
          }
        }
        Err(_) => {
          println!("Build of OpenMP code failed.");
          return false;
        },
      }
    },
    Err(_) => {
      println!("Build of OpenMP code failed.");
      return false;
    }
  }

  true
}
