pub fn mb(num_bytes: usize) -> String {
  let bytes_in_mb = ((1 as usize) << 20) as f64;
  let num_mb = num_bytes as f64 / bytes_in_mb;
  format!("{:.2}", num_mb)
}