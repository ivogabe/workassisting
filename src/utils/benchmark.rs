use core::fmt::Debug;
use std::time;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use crate::utils;

pub struct Benchmarker<T> {
  chart_style: ChartStyle,
  name: String,
  reference_time: u64,
  expected: T,
  output: Vec<(String, u32, bool, Vec<f32>)>
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ChartStyle {
  Left,
  LeftWithKey,
  Right
}

pub fn benchmark<T: Debug + Eq, Ref: FnMut() -> T>(chart_style: ChartStyle, name: &str, reference: Ref) -> Benchmarker<T> {
  println!("");
  println!("Benchmark {}", name);
  let (expected, reference_time) = time(10, reference);
  println!("Sequential   {} ms", reference_time / 1000);
  Benchmarker{ chart_style, name: name.to_owned(), reference_time, expected, output: vec![] }
}

impl<T: Copy + Debug + Eq + Send> Benchmarker<T> {
  pub fn rayon<Par: FnMut() -> T + Sync + Send>(self, label: Option<&str>, mut parallel: Par) -> Self {
    let string;
    let name = if let Some(l) = label {
      string = "Rayon (".to_owned() + l + ")";
      &string
    } else {
      "Rayon"
    };
    self.parallel(name, 1, false, |thread_count|
      utils::rayon::run_in_pool(thread_count, || { parallel() })
    )
  }

  pub fn naive_parallel<Par: Fn(usize, bool) -> T>(self, parallel: Par) -> Self {
    self.parallel("Naive", 2, false, |thread_count| parallel(thread_count, false))
      .parallel("Naive (pinned)", 3, false, |thread_count| parallel(thread_count, true))
  }

  pub fn work_stealing<Par: FnMut(usize) -> T>(self, parallel: Par) -> Self {
    self.parallel("Work stealing", 5, false, parallel)
  }

  pub fn our<Par: FnMut(usize) -> T>(self, parallel: Par) -> Self {
    self.parallel("Our", 6, true, parallel)
  }

  pub fn our_fixed_size<Par: FnMut(usize) -> T>(self, parallel: Par) -> Self {
    self.parallel("Our (specialized loop)", 4, true, parallel)
  }

  pub fn parallel<Par: FnMut(usize) -> T>(mut self, name: &str, chart_line_style: u32, our: bool, mut parallel: Par) -> Self {
    println!("{}", name);
    let mut results = vec![];
    for thread_count in [1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32] {
      if thread_count > affinity::get_core_num() {
        break;
      }

      let (value, time) = time(20, || parallel(thread_count));
      assert_eq!(self.expected, value);
      let relative = self.reference_time as f32 / time as f32;
      results.push(relative);
      println!("  {:02} threads {} ms ({:.2}x)", thread_count, time / 1000, relative);
    }
    self.output.push((name.to_owned(), chart_line_style, our, results));
    self
  }
}

impl<T> Drop for Benchmarker<T> {
  fn drop(&mut self) {
    std::fs::create_dir_all("./results").unwrap();
    let filename = "./results/".to_owned() + &self.name.replace(' ', "_").replace('(', "").replace(')', "").replace('=', "_");

    let file_gnuplot = File::create(filename.clone() + ".gnuplot").unwrap();
    let mut gnuplot = BufWriter::new(&file_gnuplot);
    writeln!(&mut gnuplot, "set title \"{}\"", self.name).unwrap();
    writeln!(&mut gnuplot, "set terminal pdf size {},2.6", if self.chart_style == ChartStyle::Right {2.3} else {2.6}).unwrap();
    writeln!(&mut gnuplot, "set output \"{}\"", filename.clone() + ".pdf").unwrap();
    if self.chart_style == ChartStyle::LeftWithKey {
      writeln!(&mut gnuplot, "set key on").unwrap();
      writeln!(&mut gnuplot, "set key top left Left reverse").unwrap();
    } else {
      writeln!(&mut gnuplot, "set key off").unwrap();
    }
    writeln!(&mut gnuplot, "set xrange [1:32]").unwrap();
    writeln!(&mut gnuplot, "set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)").unwrap();
    writeln!(&mut gnuplot, "set xlabel \"Threads\"").unwrap();
    writeln!(&mut gnuplot, "set yrange [0:18]").unwrap();
    if self.chart_style == ChartStyle::Right {
      writeln!(&mut gnuplot, "set format y \"\"").unwrap();
    } else {
      writeln!(&mut gnuplot, "set ylabel \"Speedup\"").unwrap();
    }

    write!(&mut gnuplot, "plot ").unwrap();
    for (idx, result) in self.output.iter().enumerate() {
      if idx != 0 {
        write!(&mut gnuplot, ", \\\n  ").unwrap();
      }
      write!(&mut gnuplot, "'{}.dat' using 1:{} title \"{}\" ls {} lw 1 pointsize {} with linespoints", filename, idx+2, result.0, result.1, if result.2 { 0.7 } else { 0.6 }).unwrap();
    }
    writeln!(&mut gnuplot, "").unwrap();

    let file_data = File::create(filename.clone() + ".dat").unwrap();
    let mut writer = BufWriter::new(&file_data);
    write!(&mut writer, "# Benchmark {}\n", self.name).unwrap();
    write!(&mut writer, "# Speedup compared to a sequential implementation.\n").unwrap();
    
    // Header
    write!(&mut writer, "# NCPU").unwrap();
    for result in &self.output {
      write!(&mut writer, "\t{}", result.0).unwrap();
    }
    write!(&mut writer, "\n").unwrap();

    for (idx, thread_count) in [1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32].iter().enumerate() {
      write!(&mut writer, "{}", thread_count).unwrap();
      for result in &self.output {
        if idx < result.3.len() {
          write!(&mut writer, "\t{}", result.3[idx]).unwrap();
        } else {
          write!(&mut writer, "\t").unwrap();
        }
      }
      write!(&mut writer, "\n").unwrap();
    }

    std::process::Command::new("gnuplot")
      .arg(filename + ".gnuplot")
      .spawn()
      .expect("gnuplot failed");
  }
}

pub fn time<T: Debug + Eq, F: FnMut() -> T>(runs: usize, mut f: F) -> (T, u64) {
  let first = f();
  
  let timer = time::Instant::now();
  for _ in 0 .. runs {
    assert_eq!(first, f());
  }

  (first, (timer.elapsed().as_micros() / runs as u128) as u64)
}
