pub fn mb(num_bytes: usize) -> String {
  let bytes_in_mb = ((1 as usize) << 20) as f64;
  let num_mb = num_bytes as f64 / bytes_in_mb;
  format!("{:.2}", num_mb)
}

/// Hint the CPU to start loading the cache line containing p. No-op on non-x86_64 targets.
#[inline(always)]
pub fn prefetch<T>(p: *const T) {
  #[cfg(target_arch = "x86_64")]
  unsafe {
    use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
    _mm_prefetch::<_MM_HINT_T0>(p as *const i8);
  }
  #[cfg(not(target_arch = "x86_64"))]
  let _ = p;
}
