pub fn run_in_pool<T: Send, F: FnMut() -> T + Send>(thread_count: usize, f: F) -> T {
  match rayon::ThreadPoolBuilder::new().num_threads(thread_count).build() {
    Err(e) => panic!("Rayon failed: {}", e),
    Ok(pool) => {
      pool.install(f)
    }
  }
}
