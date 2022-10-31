
use crate::{hmap::HMap, coalg::{init_partition_ids_unsafe, repartition_all_unsafe64}, binrep::CReader};

#[cfg(test)]
use crate::{io::read_boa_txt, renumber};

fn count_parts(sigs: &[u64]) -> usize {
  let mut sigs2 = sigs.to_vec();
  sigs2.sort_unstable();
  let mut freqs : HMap<u32,u32> = HMap::default();
  let mut lastsig:u64 = 0;
  let mut count = 0;
  for &sig in &sigs2 {
      if sig == lastsig {
          count += 1;
      }else{
          if freqs.contains_key(&count){
              freqs.insert(count, freqs[&count]+1);
              // if count == 2 {
                  // println!("Alarm: {:?} {:?} {:?} {:?}", sig, lastsig, count, freqs);
              // }
          }else{
              freqs.insert(count, 1);
          }
          lastsig = sig;
          count = 1;
      }
  }
  freqs.remove(&0);
  println!("Partition freqs: {:?}", std::collections::BTreeMap::from_iter(freqs.iter()));
  sigs2.dedup();
  return sigs2.len();
}

pub fn partref_naive(data: &[u8], r: &CReader) -> Vec<u32> {
  let mut ids = init_partition_ids_unsafe(data, r);
  let mut part_count = count_parts(&ids);
  println!("Initial number of partitions/total states: {}/{}", part_count, ids.len());
  for iter in 0..99999999 {
      // let start_time = SystemTime::now();
      let new_ids = repartition_all_unsafe64(data, r, &ids);
      let new_part_count = count_parts(&new_ids);
      println!("Iteration: {}, number of partitions/total states: {}/{}", iter, new_part_count, ids.len());
      if new_part_count == new_ids.len() || new_part_count == part_count {
          println!("Number of iterations: {}", iter+1);
          return new_ids.iter().map(|id| *id as u32).collect();
      } else {
          ids = new_ids;
          part_count = new_part_count;
      }
  }
  panic!("Ran out of iterations.")
}

#[test]
fn test_partref_naive() {
  let (data,r) = read_boa_txt("tests/test1.boa.txt");
  let ids = renumber(&partref_naive(&data,&r));
  assert_eq!(&ids, &vec![0,0,1,1,2,3,3,4]);
}