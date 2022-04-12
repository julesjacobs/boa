Boa: Fast binary generic coalgebraic partition refinement
----------------------------------------------------------

Primitive: given vec of states, compute refined partition (list of integer IDs)

Data of algorithm:
- binary data for states
- start of binary storage for each state
- back edges for each state

State of algorithm:
- worklist of partitions
  + list of all states in partition
  + list of dirty states in partition
- dirty bit array
- current ID assignment

Initial state:
- worklist = single partition of all states
- dirty bit array = everything dirty
- current ID assignment = all zero

Iteration:
- Take a partition off the worklist
- Mark all states in this partition clean
- Compute refined partition (list of integer IDs)
- Reassign ID assignment
  + Use old ID for the largest subpartition
  + For each state whose ID was changed, mark all back edges dirty
    (set dirty bit & put partition on worklist if wasn't already dirty)

# Computing refined partition

## Hash based

Compute hash for each state.
Need to do something for Set/Add/Or/Max.

## Exact

canon(x) := x if is_state(x)
canon(x) := listtable.canon(lookup(x)) if x is a list
canon(x) := settable.canon(lookup(x)) if x is a set
canon(x) := addtable.canon(lookup(x)) if x is an add
canon(x) := uniontable.canon(lookup(x)) if x is an union
canon(x) := maxtable.canon(lookup(x)) if x is an max

# Algorithm

## Recompute everything in-order

- Recompute everything in-order using exact partitioning, until no change in IDs
- Recompute everything in-order using hash partitioning, renumber, and iterate until no change, then do one exact iteration per group
- Recompute everything in-order using hash partitioning, check if any partition split (keep track of partitions)

## Recompute per partition

- Without back edges dirty set: continue iterating over partitions until nothing split for an entire iteration
- With back edges dirty set


# Partition refinement data structure for true n log n algorithm

The main difficulty is that we need to be able to obtain all states of the partition in case the partition that the clean part belongs to is not the largest one.

Maintain partition as buffer of states + pointers into that structure telling where each partition (and dirty part) starts and ends.
To mark state dirty, advance dirty pointer and swap the dirty state in there.

Data:
- position[state]         -- position of the state in the buffer
- buffer[i]               -- which state is at position i in the buffer
- partition_id[state]     -- which partition the state is in
- partition[id]           -- triple (start_dirty, end_dirty, end_clean) of indices into the buffer
- worklist                -- list of partitions with non-empty dirty part

fn mark_dirty(state) {
  id = partition_id[state]
  (start_dirty, end_dirty, end_clean) = partition[id]
  if start_dirty == end_dirty { worklist.push(id) }

  if state < end_dirty { return } // state is already dirty
  else { // state is not yet dirty
    i = partition[id].end_dirty // we will put the state here
    partition[id].end_dirty += 1
    other_state = buffer[i]
    j = position[state]
    position[state] = i
    position[other_state] = j
    buffer[i] = state
    buffer[j] = other_state
  }
}

fn refine(id, signatures) {
  // signatures are the signatures for the dirty states if all states are dirty
  // otherwise the last element of signatures is the signature for the clean states
}

fn get_refiners(id) -> &[State] {
  // slice of states to compute signatures for
  (start_dirty,end_dirty,end_clean) = partition[id]
  if end_dirty == end_clean { end = end_dirty }
  else { end = end_dirty + 1 }
  return &buffer[start_dirty..end]
}




Uniform representation (simpler!).

32 bits per value
- first bit is 0: state ref
- first bit is 1: first byte is header (128 possibilities)
- constant, or (tuple|bag|set|monoid) header + list of children
- in construct with list of children, header stores length in last 3rd and 4th byte (16 bits)
  2nd byte is unused, but does participate in hashing => can be used for constructor tag if value fits in one byte
- in case of monoid, store list of monoid values first (of the given length)
  in case of distribution, skip the first monoid value since it must sum to 1? (can skip this at first, just use the R/float32 monoid)
- can also support polynomial arithmetic expressions maybe

e ::= N | @N | Col[t]{e,...} | Mon[t]{e:m,...}
Col ::= Set | Bag | List
Mon ::= Z | R | Nmax | Bor

R[22]{e1: 23.3, e2: 43.2, ...}

- Write conversion program xxx.boa <-> xxx.boa.txt
- Write boa program itself
- boa xxx.boa            // compute equivalence classes
- boa xxx.boa.txt        // compute equivalence classes
- boa -c xxx.boa         // convert
- boa -c xxx.boa.txt     // convert

struct buf {
  size_t size;
  size_t capacity;
  void* data;
}

read(filename) : buf
write(filename, buf)
put8(buf,chr)
put32(buf,int)


Naive algorithm:
- initialize all state labels to 0
- hash all states and assign hash as state label
- repeat until convergence
- deal with hash collisions

Less naive algorithm:
- same but only update labels that actually changed
- assign old label to greatest subpartition
- data structures needed:
  + predecessor states of each state (this is the big one)
  + array mapping state to partition
  + array mapping state to dirty bit
  + work list of partitions, with a list of dirty states in each partition
- algorithm:
  + take partition from work list
  + compute hash of all dirty states
  + choose labels, choosing the old label for the largest subpartition
  + mark all affected states as dirty


What do we need per state?
- compute its hash
- compare with other state (for hash collision dealing) [optional, apparently]

To compute hash:
- For monoid valued lists:
  Compute hashes for each element of the list
  Now we have an array of monoid values and an array of hashes
  Sort by hash, compute sums, hash result
To do precise labeling:
- Insert things into hash table

