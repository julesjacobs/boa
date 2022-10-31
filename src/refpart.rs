use std::collections::VecDeque;

use datasize::DataSize;

fn cumsum_mut(xs: &mut [u32]) {
  let mut sum = 0;
  for x in xs.iter_mut() {
      sum += *x;
      *x = sum;
  }
}

#[test]
fn test_cumsum_mut() {
  let mut xs = vec![2,3,1,2,0,4];
  cumsum_mut(&mut xs);
  assert_eq!(xs, vec![2,5,6,8,8,12]);
}

fn cumsum(xs: &[u32]) -> Vec<u32> {
  let mut xs = xs.to_vec();
  cumsum_mut(&mut xs);
  return xs;
}

#[test]
fn test_cumsum() {
  let xs = vec![2,3,1,2,0,4];
  assert_eq!(cumsum(&xs), vec![2,5,6,8,8,12]);
}


fn counts_vec(xs: &[u32]) -> Vec<u32> {
  let mut counts = vec![];
  for &x in xs {
      while x as usize >= counts.len() { counts.push(0); }
      counts[x as usize] += 1;
  }
  return counts;
}

#[test]
fn test_counts_vec() {
  let counts = counts_vec(&vec![0,0,1,1,3,4,5,5,5]);
  assert_eq!(counts[0],2);
  assert_eq!(counts[1],2);
  assert_eq!(counts[3],1);
  assert_eq!(counts[4],1);
  assert_eq!(counts[5],3);
}

fn index_of_max(counts: &[u32]) -> usize {
  let mut i_max = usize::MAX;
  let mut v_max = 0;
  for i in 0..counts.len() {
      if counts[i] >= v_max {
          i_max = i;
          v_max = counts[i];
      }
  }
  return i_max
}

#[test]
fn test_index_of_max() {
  assert_eq!(index_of_max(&vec![0,3,1,2,3,4,3]), 5);
}

type State = u32;

#[derive(DataSize)]
pub struct RefinablePartition {
  pub buffer: Vec<State>, // buffer of states (partitioned)
  pub position: Vec<u32>, // position of each state in the buffer
  pub state2block: Vec<u32>, // send each state to the surrounding block
  pub partition: Vec<(u32,u32,u32)>, // vector of blocks (start, mid, end) where the states in start..mid are dirty and mid..end are clean. all intervals are half-open (inclusive start, exclusive end).
  pub worklist: VecDeque<u32>, // worklist: blocks with at least one dirty state
}

impl RefinablePartition {
  pub fn new(num_states: u32) -> RefinablePartition {
      RefinablePartition {
          buffer: (0..num_states).collect(),
          position: (0..num_states).collect(),
          state2block: vec![0;num_states as usize],
          partition: vec![(0, 0, num_states)], // for partition (start, mid, end), the states start..mid are clean and mid..end are dirty
          worklist: VecDeque::from(vec![0])
      }
  }

  /// Mark the state as dirty, putting its partition on the worklist if necessary
  /// Time complexity: O(1)
  pub fn mark_dirty(self: &mut RefinablePartition, state: State) {
      // unsafe {
      //     let id = *self.state2block.get_unchecked(state as usize);
      //     let pos = *self.position.get_unchecked(state as usize);
      //     let (start, mid, end) = *self.partition.get_unchecked(id as usize);
      //     // println!("mark_dirty(_,{}): id={}, pos={}, part={:?}", state, id, pos, (start,mid,end));
      //     if end - start <= 1 { return } // don't need to mark states dirty if they are in a singleton partition
      //     if mid <= pos { // state is already dirty
      //         return
      //     }
      //     if mid == end { // no dirty states in partition yet, so put it onto worklist
      //         self.worklist.push_back(id)
      //     }
      //     self.partition.get_unchecked_mut(id as usize).1 -= 1; // decrement the dirty states marker to make space
      //     let other_state = *self.buffer.get_unchecked(mid as usize - 1); // the state that we will swap
      //     *self.position.get_unchecked_mut(other_state as usize) = pos;
      //     *self.position.get_unchecked_mut(state as usize) = mid;
      //     *self.buffer.get_unchecked_mut(pos as usize) = other_state;
      //     *self.buffer.get_unchecked_mut(mid as usize - 1) = state;
      // }
      let id = self.state2block[state as usize];
      let pos = self.position[state as usize];
      let (start, mid, end) = self.partition[id as usize];
      // println!("mark_dirty(_,{}): id={}, pos={}, part={:?}", state, id, pos, (start,mid,end));
      if end - start <= 1 { return } // don't need to mark states dirty if they are in a singleton partition
      if mid <= pos { // state is already dirty
          return
      }
      if mid == end { // no dirty states in partition yet, so put it onto worklist
          self.worklist.push_back(id)
      }
      self.partition[id as usize].1 -= 1; // decrement the dirty states marker to make space
      let other_state = self.buffer[mid as usize - 1]; // the state that we will swap
      self.position[other_state as usize] = pos;
      self.position[state as usize] = mid - 1;
      self.buffer[pos as usize] = other_state;
      self.buffer[mid as usize - 1] = state;
  }

  /// Determine slice of states to compute signatures for.
  /// Includes one clean state at the start if there are any clean states.
  /// Time complexity: O(1)
  pub fn refiners(self: &RefinablePartition, id: u32) -> &[State] {
      let (start, mid, end) = self.partition[id as usize];
      if start == mid { // no clean states
          return &self.buffer[start as usize..end as usize]
      } else { // there are clean states
          return &self.buffer[(mid-1) as usize..end as usize]
      }
  }

  /// TODO: Optimisation: assign the old ID to the block with the fewest predecessors
  /// Time complexity: O(signatures.len())
  /// Returns vector of new partition ids
  /// Signatures are assumed to be 0..n with the first starting with 0
  pub fn refine(self: &mut RefinablePartition, partition_id: u32, signatures: &[u32]) -> Vec<u32> {
      // let signatures = renumber(signatures); // Renumber signatures to be 0..n. This makes the sig of the clean states 0 if there are any.

      // compute the occurrence counts of each of the signatures
      let mut counts = counts_vec(&signatures);

      let (start,mid,end) = self.partition[partition_id as usize];
      if start < mid { counts[0] += mid - start - 1 } // add count of clean part
      // println!("refine: {:?}", &counts);

      // sort the relevant part of self.buffer by signature
      // also restores invariant for self.position and self.partition_id
      let largest_partition = index_of_max(&counts) as u32;
      let next_available_partition_id = self.partition.len() as u32;

      let mut cum_counts = cumsum(&counts);
      let original_states = self.refiners(partition_id).to_vec();

      for i in 0..original_states.len() {
          let sig = signatures[i];
          let state = original_states[i];
          cum_counts[sig as usize] -= 1;
          let j = start+cum_counts[sig as usize];
          self.buffer[j as usize] = state;
          self.position[state as usize] = j;

          if sig != largest_partition {
              let new_sig = next_available_partition_id + if sig < largest_partition { sig } else { sig - 1 };
              self.state2block[state as usize] = new_sig;
          }
      }

      if largest_partition != 0 {
          // need to relabel the clean states
          for i in start..mid {
              let state = self.buffer[i as usize];
              self.state2block[state as usize] = next_available_partition_id;
          }
      }

      if start < mid { cum_counts[0] -= mid - start - 1 }
      debug_assert_eq!(cum_counts[0],0);
      debug_assert_eq!(cum_counts[cum_counts.len()-1] + counts[counts.len()-1], end - start);


      // we will return vector of the new partitions
      let mut new_partitions: Vec<u32> = vec![];

      // restore invariant of self.partition
      for sig in 0..counts.len() as u32 {
          let new_start = start+cum_counts[sig as usize];
          let new_end = start+cum_counts[sig as usize]+counts[sig as usize];
          let new_part = (new_start, new_end, new_end); // all states are clean now (but may be marked dirty later)
          if sig == largest_partition {
              self.partition[partition_id as usize] = new_part;
          } else {
              new_partitions.push(self.partition.len() as u32);
              self.partition.push(new_part);
          }
      }

      return new_partitions;
  }
}