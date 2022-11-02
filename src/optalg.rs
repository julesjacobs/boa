use std::time::SystemTime;

use datasize::data_size;

#[cfg(test)]
use crate::{naivealg::partref_naive, io::read_boa_txt};

use crate::{binrep::CReader, refpart::RefinablePartition, coalg::repartition_unsafe, coalg::Coalg, renumber, util, };

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
  while let Some(block_id) = if false { partition.worklist.pop_front() } else { partition.worklist.pop_back() } {

      // let (start,mid,end) = partition.partition[block_id as usize];
      // println!("iteration={} #partitions={} #worklist={} clean={} dirty={}", iters, partition.partition.len(), partition.worklist.len(), mid-start, end-mid);

      let states = partition.refiners(block_id);
      // println!("states = {:?}", states);
      let signatures = renumber::<u64>(&repartition_unsafe(&coa, states, &partition.state2block));
      // println!("partition id = {:?}, partition = {:?}, states = {:?}, sigs = {:?}", block_id, partition.partition[block_id as usize], states, &signatures);
      let new_blocks = partition.refine(block_id, &signatures);
      // println!("shrunk partition = {:?}, new partitions = {:?}, buffer = {:?}", partition.partition[block_id as usize], &new_partitions.iter().map(|pid| partition.partition[*pid as usize]).collect::<Vec<(u32,u32,u32)>>(), &partition.buffer);
      for predecessor_block in new_blocks {
          // mark dirty all predecessors of states in this partition
          // let part_debug = partition.partition[predecessor_block as usize];
          let (start,_, end) = partition.partition[predecessor_block as usize];
          let states = partition.buffer[start as usize..end as usize].to_vec();
          for state in states {
              for &state2 in coa.state_backrefs(state) {
                  // println!("state {} marks state {} as dirty (new partition: {:?} id: {})", state, state2, &part_debug, predecessor_block);
                  partition.mark_dirty(state2);
              }
          }
      }
      iters += 1;
  }
  println!("iters: {} ", iters);
  // println!("coalg_input_mb: {}", util::mb(data_size(&coa.data)));
  println!("coalg_refs_mb: {}", util::mb(data_size(&coa) - data_size(&coa.data)));
  println!("refpart_mb: {}", util::mb(data_size(&partition)));
  return partition.state2block;
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