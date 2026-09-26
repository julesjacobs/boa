use std::time::SystemTime;

use datasize::data_size;

#[cfg(test)]
use crate::{
  binrep::{CWriter, Node, LIST_TYP},
  io::read_boa_txt,
  naivealg::partref_naive,
};

use crate::{binrep::CReader, refpart::RefinablePartition, coalg::repartition_into, coalg::Coalg, hmap::HMap, renumber, util, };

fn partref_nlogn_raw(data: Vec<u8>, r: CReader) -> Vec<u32> {
  // println!("===================== Starting partref_nlogn");
  // panic!("Stopped");
  let start_time = SystemTime::now();
  // print!("Initializing backrefs...");
  let coa = Coalg::new(data, r);
  let backrefs_time = start_time.elapsed().unwrap();
  println!("backrefs_time_s: {}", backrefs_time.as_secs_f32());
  // coa.dump();
  // coa.dump_backrefs();
  println!("m_edges: {}", coa.backrefs.len());
  let mut iters = 0;
  let mut partition = RefinablePartition::new(coa.num_states());

  // Buffers reused across iterations, so that the main loop does not allocate.
  let mut raw_sigs: Vec<u64> = vec![];
  let mut scratch: Vec<(u64,u64)> = vec![];
  let mut renumberer = Renumberer::default();
  let mut signatures: Vec<u32> = vec![];
  let mut moved_states: Vec<u32> = vec![];

  let start_time_iters = SystemTime::now();
  while let Some(block_id) = partition.worklist.pop_back() {
      let states = partition.refiners(block_id);
      repartition_into(&coa, states, &partition.state2block, &mut raw_sigs, &mut scratch);
      renumberer.renumber(&raw_sigs, &mut signatures);
      release_if_large(&mut raw_sigs);
      let new_blocks = partition.refine(block_id, &signatures);
      release_if_large(&mut signatures);

      // mark dirty all predecessors of states that moved to a new block
      // (copied first, because mark_dirty reorders the states within blocks)
      moved_states.clear();
      for predecessor_block in new_blocks {
          let (start,_, end) = partition.partition[predecessor_block as usize];
          moved_states.extend_from_slice(&partition.buffer[start as usize..end as usize]);
      }
      for &state in &moved_states {
          for &state2 in coa.state_backrefs(state) {
              partition.mark_dirty(state2);
          }
      }
      release_if_large(&mut moved_states);
      iters += 1;
  }
  println!("iters: {} ", iters);
  // println!("coalg_input_mb: {}", util::mb(data_size(&coa.data)));
  println!("coalg_refs_mb: {}", util::mb(data_size(&coa) - data_size(&coa.data)));
  println!("refpart_mb: {}", util::mb(data_size(&partition)));
  let iter_time = start_time_iters.elapsed().unwrap();
  println!("iter_time_s: {}", iter_time.as_secs_f32());
  let selfreport_time = start_time.elapsed().unwrap();
  println!("selfreport_time_s: {}", selfreport_time.as_secs_f32());
  return partition.state2block;
}

/// Buffers above this many elements are freed after use rather than kept around,
/// so that the few very large iterations don't increase the peak memory use.
pub const LARGE_BUFFER: usize = 1 << 16;

pub fn release_if_large<T>(v: &mut Vec<T>) {
  if v.capacity() > LARGE_BUFFER { *v = Vec::new(); }
}

/// Renumbers signatures to 0..k in order of first occurrence, reusing its memory across calls.
#[derive(Default)]
struct Renumberer {
  map: HMap<u64,u32>,
}

impl Renumberer {
  const SMALL: usize = 16;

  fn renumber(&mut self, sigs: &[u64], out: &mut Vec<u32>) {
      out.clear();
      out.reserve(sigs.len());
      if sigs.len() <= Self::SMALL {
          // Linear search is faster than hashing for small blocks.
          let mut seen = [0u64; Self::SMALL];
          let mut num_seen = 0;
          for &sig in sigs {
              let id = match seen[..num_seen].iter().position(|&s| s == sig) {
                  Some(id) => id,
                  None => { seen[num_seen] = sig; num_seen += 1; num_seen - 1 }
              };
              out.push(id as u32);
          }
          return;
      }
      // Clearing a map costs time proportional to its capacity, so drop it if it's much too big.
      if self.map.capacity() > 4 * sigs.len() {
          self.map = HMap::default();
      } else {
          self.map.clear();
      }
      self.map.reserve(sigs.len());
      for &sig in sigs {
          let next = self.map.len() as u32;
          out.push(*self.map.entry(sig).or_insert(next));
      }
      if self.map.capacity() > LARGE_BUFFER { self.map = HMap::default(); }
  }
}

#[test]
fn test_renumberer_matches_renumber() {
  let mut renumberer = Renumberer::default();
  let mut out = vec![];
  for len in [0, 1, 5, 16, 17, 100, 3] {
      let sigs: Vec<u64> = (0..len).map(|i| (i * 7 % 5) as u64 * 1_000_000_007).collect();
      renumberer.renumber(&sigs, &mut out);
      assert_eq!(out, renumber(&sigs));
  }
}

pub fn partref_nlogn(data: Vec<u8>, r: CReader) -> Vec<u32> {
  let ids = partref_nlogn_raw(data, r);
  return renumber(&ids);
}

#[test]
fn test_partref_nlogn() {
  // List[0]{@0,@1}
  // List[0]{@1,@1}
  // List[1]{@0,@0}
  // List[1]{@0,@0}
  // List[1]{@3,@4}
  // Add[0]{@0:1,@1:1}
  // Add[0]{@0:2}
  // Add[0]{@0:2,@1:1}
  let (data,r) = read_boa_txt("tests/test1.boa.txt");
  let ids1 = partref_naive(&data, &r);
  let ids2 = partref_nlogn(data, r);
  assert_eq!(&renumber(&ids1), &ids2);

  let (data,r) = read_boa_txt("tests/test2.boa.txt");
  let ids = partref_nlogn(data, r);
  assert_eq!(&ids, &vec![0,1,2,3,4,5]);
}

#[test]
fn test_partref_wlan() {
  let filename = "tests/small/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_4.boa.txt";
  let (data,r) = read_boa_txt(&filename);
  let ids = partref_nlogn(data, r);
  assert_eq!(*ids.iter().max().unwrap(), 107864);

  let filename = "tests/wlan1_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1408676_1963522_roundrobin_32.boa.txt";
  let (data, r) = read_boa_txt(&filename);
  let ids = partref_nlogn(data, r);
  assert_eq!(*ids.iter().max().unwrap(), 243324);
}

#[cfg(test)]
fn assert_dense_partition(ids: &[u32]) {
  let mut unique = ids.to_vec();
  unique.sort_unstable();
  unique.dedup();
  assert_eq!(unique, (0..unique.len() as u32).collect::<Vec<_>>());
}

#[cfg(test)]
fn assert_same_partition(left: &[u32], right: &[u32]) {
  assert_eq!(left.len(), right.len());
  for i in 0..left.len() {
      for j in 0..left.len() {
          assert_eq!(left[i] == left[j], right[i] == right[j], "states {i} and {j}");
      }
  }
}

#[cfg(test)]
fn compare_algorithms(data: Vec<u8>, reader: CReader) {
  let naive = partref_naive(&data, &reader);
  let optimized = partref_nlogn(data, reader);
  assert_dense_partition(&naive);
  assert_dense_partition(&optimized);
  assert_same_partition(&naive, &optimized);
}

#[test]
fn paper_fixtures_match_naive_refinement() {
  let mut filenames = glob::glob("tests/paper_test*.boa.txt")
      .unwrap()
      .map(Result::unwrap)
      .collect::<Vec<_>>();
  filenames.extend(["tests/test1.boa.txt".into(), "tests/test2.boa.txt".into()]);
  filenames.sort();

  for filename in filenames {
      let (data, reader) = read_boa_txt(&filename);
      compare_algorithms(data, reader);
  }
}

#[cfg(test)]
struct TestRng(u64);

#[cfg(test)]
impl TestRng {
  fn next(&mut self) -> u64 {
      let mut value = self.0;
      value ^= value << 13;
      value ^= value >> 7;
      value ^= value << 17;
      self.0 = value;
      value
  }

  fn usize(&mut self, upper: usize) -> usize {
      self.next() as usize % upper
  }
}

#[test]
fn generated_graphs_match_naive_refinement() {
  let mut rng = TestRng(0x5eed_1234_abcd_9876);
  for _ in 0..100 {
      let num_states = 1 + rng.usize(40);
      let mut writer = CWriter::new();
      for _ in 0..num_states {
          let node = Node::Coll(
              LIST_TYP,
              rng.usize(8) as u8,
              (0..rng.usize(6))
                  .map(|_| Node::State(rng.usize(num_states) as u32))
                  .collect(),
          );
          node.write(&mut writer);
      }
      let (data, reader) = writer.finish();
      compare_algorithms(data, reader);
  }
}
