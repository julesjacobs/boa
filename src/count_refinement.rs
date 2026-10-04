use fxhash::FxHashMap;

pub(crate) trait CountRefinementGraph {
    fn dense_rows(&self) -> Option<&crate::dense_stability::DenseRows> {
        None
    }
    fn num_states(&self) -> usize;
    fn initial_signature(&self, state: u32) -> (u32, u32);
    fn visit_predecessors(&self, state: u32, visit: impl FnMut(u32));
    fn visit_successors(&self, state: u32, visit: impl FnMut(u32));
}

/// Exact refinement of unit-weight multisets, starting from tags and outdegrees.
pub(crate) fn refine(graph: &impl CountRefinementGraph) -> Vec<u32> {
    refine_impl(graph, false, mix, 100_000)
}

pub(crate) fn refine_hybrid(graph: &impl CountRefinementGraph) -> Vec<u32> {
    let compact = if graph.num_states() <= crate::stack_compact_refinement::MAX_STATES {
        crate::stack_compact_refinement::refine(graph)
    } else {
        crate::compact_refinement::try_refine(graph)
    };
    if let Some(partition) = compact {
        return partition;
    }
    refine_impl(graph, true, mix, 100_000)
}

fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn refine_impl(
    graph: &impl CountRefinementGraph,
    hybrid: bool,
    seed_hash: impl Fn(u64) -> u64,
    seed_min_states: usize,
) -> Vec<u32> {
    let n = graph.num_states();
    if n == 0 {
        return Vec::new();
    }

    let first_signature = graph.initial_signature(0);
    let mut prefix = 1;
    while prefix < n && graph.initial_signature(prefix as u32) == first_signature {
        prefix += 1;
    }
    if prefix == n {
        return vec![0; n];
    }
    let first_tag = first_signature.0;
    let mut degree_ids_for_first_tag = vec![u32::MAX; first_signature.1 as usize + 1];
    degree_ids_for_first_tag[first_signature.1 as usize] = 0;
    let mut second_tag = None;
    let mut degree_ids_for_second_tag = [u32::MAX; 65];
    let mut other_tag_ids = FxHashMap::default();
    let mut sizes = vec![prefix as u32];
    let mut blocks = Vec::with_capacity(n);
    blocks.resize(prefix, 0);
    let mut max_degree = first_signature.1;
    let mut total_degree = first_signature.1 as u64 * prefix as u64;
    for state in prefix..n {
        let signature = graph.initial_signature(state as u32);
        max_degree = max_degree.max(signature.1);
        total_degree += signature.1 as u64;
        let next_id = sizes.len() as u32;
        let id = if signature.0 == first_tag {
            degree_ids_for_first_tag.resize(
                degree_ids_for_first_tag.len().max(signature.1 as usize + 1),
                u32::MAX,
            );
            let entry = &mut degree_ids_for_first_tag[signature.1 as usize];
            if *entry == u32::MAX {
                *entry = next_id;
                sizes.push(0);
            }
            *entry
        } else if signature.1 <= 64 && second_tag.is_none_or(|tag| tag == signature.0) {
            second_tag = Some(signature.0);
            let entry = &mut degree_ids_for_second_tag[signature.1 as usize];
            if *entry == u32::MAX {
                *entry = next_id;
                sizes.push(0);
            }
            *entry
        } else {
            *other_tag_ids.entry(signature).or_insert_with(|| {
                sizes.push(0);
                next_id
            })
        };
        sizes[id as usize] += 1;
        blocks.push(id);
    }
    if sizes.len() == 1 || sizes.len() == n || max_degree == 0 {
        return blocks;
    }

    if hybrid
        && sizes.len() <= 4
        && graph
            .dense_rows()
            .is_some_and(|rows| rows.certify(&blocks, sizes.len()))
    {
        return blocks;
    }

    if hybrid && sizes.len() <= 4 {
        let stable = if n >= 100_000 {
            crate::small_stability::small_initial_stable(graph, &blocks, &sizes, max_degree)
        } else if max_degree > 64 {
            crate::small_stability::small_initial_stable_by_volume(
                graph, &blocks, &sizes, max_degree,
            )
        } else if total_degree <= n as u64 * 8 {
            exact_small_stable(graph, &blocks, sizes.len())
        } else {
            false
        };
        if stable {
            return blocks;
        }
    }

    let seed_eligible = hybrid
        && n >= seed_min_states
        && total_degree >= n as u64 * 4
        && total_degree <= n as u64 * 16;
    if seed_eligible {
        let mut next_blocks = vec![0; n];
        let mut signature_ids = FxHashMap::default();
        signature_ids.reserve(n);
        let mut hashes = vec![0_u64; n];
        let mut hashes_initialized = 0;
        for _ in 0..8 {
            for block in hashes_initialized..sizes.len() {
                hashes[block] = seed_hash(block as u64);
            }
            hashes_initialized = sizes.len();
            signature_ids.clear();
            let previous_classes = sizes.len();
            let mut next_sizes = vec![0_u32; sizes.len()];
            for state in 0..n as u32 {
                let previous_block = blocks[state as usize] as usize;
                if sizes[previous_block] == 1 {
                    next_sizes[previous_block] = 1;
                    next_blocks[state as usize] = previous_block as u32;
                    continue;
                }
                let mut signature = 0_u64;
                graph.visit_successors(state, |target| {
                    signature = signature.wrapping_add(hashes[blocks[target as usize] as usize]);
                });
                let id = *signature_ids
                    .entry((previous_block as u32, signature))
                    .or_insert_with(|| {
                        if next_sizes[previous_block] == 0 {
                            previous_block as u32
                        } else {
                            let id = next_sizes.len() as u32;
                            next_sizes.push(0_u32);
                            id
                        }
                    });
                next_sizes[id as usize] += 1;
                next_blocks[state as usize] = id;
            }
            // These fingerprints only seed refinement; exact counts still decide stability.
            std::mem::swap(&mut blocks, &mut next_blocks);
            sizes = next_sizes;
            if sizes.len() == n {
                return normalize(blocks, sizes.len());
            }
            if sizes.len() == previous_classes {
                break;
            }
        }
    }

    let mut ranges = Vec::with_capacity(sizes.len());
    let mut start = 0;
    for &size in &sizes {
        ranges.push((start, start + size));
        start += size;
    }
    let mut insertion = ranges.iter().map(|&(start, _)| start).collect::<Vec<_>>();
    let mut buffer = vec![0; n];
    let mut position = vec![0; n];
    for (state, &block) in blocks.iter().enumerate() {
        let slot = &mut insertion[block as usize];
        buffer[*slot as usize] = state as u32;
        position[state] = *slot;
        *slot += 1;
    }
    if seed_eligible && exact_stable(graph, &blocks, &ranges, &buffer) {
        return normalize(blocks, sizes.len());
    }
    let largest = sizes
        .iter()
        .enumerate()
        .max_by_key(|&(_, size)| size)
        .unwrap()
        .0;
    let mut worklist = Vec::new();
    // Every block has a fixed outdegree, so counts into the omitted initial cell
    // are determined by the counts into its complement.
    for block in 0..ranges.len() {
        if block != largest {
            worklist.push(block as u32);
        }
    }
    worklist.sort_unstable_by_key(|&block| std::cmp::Reverse(sizes[block as usize]));

    let mut singleton = blocks
        .iter()
        .map(|&block| sizes[block as usize] == 1)
        .collect::<Vec<_>>();
    let mut counts = vec![0_u32; n];
    let mut marked = vec![0_u32; n];
    let mut hit_blocks = Vec::new();
    let mut touched = Vec::new();
    let mut splitter_states = Vec::new();
    let mut states = Vec::new();
    let mut frequencies = vec![0_u32; max_degree as usize + 1];
    let mut destinations = vec![0_u32; max_degree as usize + 1];
    let mut used_counts = Vec::new();
    let mut fragments = Vec::new();

    while !worklist.is_empty() && ranges.len() < n {
        let first = worklist.len().saturating_sub(8);
        let selected = (first..worklist.len())
            .min_by_key(|&index| sizes[worklist[index] as usize])
            .unwrap();
        let splitter = worklist.swap_remove(selected);
        let (start, end) = ranges[splitter as usize];

        hit_blocks.clear();
        touched.clear();
        // Tail marking can move vertices in the splitter itself.
        splitter_states.clear();
        splitter_states.extend_from_slice(&buffer[start as usize..end as usize]);
        for &target in &splitter_states {
            graph.visit_predecessors(target, |state| {
                if singleton[state as usize] {
                    return;
                }
                if counts[state as usize] == 0 {
                    touched.push(state);
                    let block = blocks[state as usize] as usize;
                    if marked[block] == 0 {
                        hit_blocks.push(block);
                    }
                    marked[block] += 1;
                    let destination = ranges[block].1 - marked[block];
                    let old_position = position[state as usize];
                    let other = buffer[destination as usize];
                    buffer[old_position as usize] = other;
                    position[other as usize] = old_position;
                    buffer[destination as usize] = state;
                    position[state as usize] = destination;
                }
                counts[state as usize] += 1;
            });
        }

        for &block in &hit_blocks {
            let (start, end) = ranges[block];
            let boundary = end - marked[block];
            marked[block] = 0;
            let first_count = counts[buffer[boundary as usize] as usize];
            let uniform = buffer[boundary as usize..end as usize]
                .iter()
                .all(|&state| counts[state as usize] == first_count);
            if uniform {
                if boundary == start {
                    continue;
                }
                let (kept, moved) = if boundary - start >= end - boundary {
                    ((start, boundary), (boundary, end))
                } else {
                    ((boundary, end), (start, boundary))
                };
                ranges[block] = kept;
                sizes[block] = kept.1 - kept.0;
                if sizes[block] == 1 {
                    singleton[buffer[kept.0 as usize] as usize] = true;
                }
                let new_block = ranges.len() as u32;
                ranges.push(moved);
                sizes.push(moved.1 - moved.0);
                if moved.1 - moved.0 == 1 {
                    singleton[buffer[moved.0 as usize] as usize] = true;
                }
                for slot in moved.0..moved.1 {
                    blocks[buffer[slot as usize] as usize] = new_block;
                }
                worklist.push(new_block);
                continue;
            }

            states.clear();
            states.extend_from_slice(&buffer[boundary as usize..end as usize]);
            used_counts.clear();
            for &state in &states {
                let count = counts[state as usize] as usize;
                if frequencies[count] == 0 {
                    used_counts.push(count);
                }
                frequencies[count] += 1;
            }

            fragments.clear();
            let mut cursor = start;
            if boundary != start {
                fragments.push((cursor, boundary));
                cursor = boundary;
            }
            for &count in &used_counts {
                destinations[count] = cursor;
                let next = cursor + frequencies[count];
                fragments.push((cursor, next));
                cursor = next;
                frequencies[count] = 0;
            }

            for &state in &states {
                let destination = &mut destinations[counts[state as usize] as usize];
                let old_position = position[state as usize];
                let other = buffer[*destination as usize];
                buffer[old_position as usize] = other;
                position[other as usize] = old_position;
                buffer[*destination as usize] = state;
                position[state as usize] = *destination;
                *destination += 1;
            }

            let largest = fragments
                .iter()
                .enumerate()
                .max_by_key(|&(_, &(start, end))| end - start)
                .unwrap()
                .0;
            ranges[block] = fragments[largest];
            sizes[block] = ranges[block].1 - ranges[block].0;
            if sizes[block] == 1 {
                singleton[buffer[ranges[block].0 as usize] as usize] = true;
            }
            // A queued cell keeps its queue entry for the largest fragment;
            // otherwise only the smaller fragments need to become splitters.
            for (index, &(start, end)) in fragments.iter().enumerate() {
                if index == largest {
                    continue;
                }
                let new_block = ranges.len() as u32;
                ranges.push((start, end));
                sizes.push(end - start);
                if end - start == 1 {
                    singleton[buffer[start as usize] as usize] = true;
                }
                for slot in start..end {
                    blocks[buffer[slot as usize] as usize] = new_block;
                }
                worklist.push(new_block);
            }
        }
        for &state in &touched {
            counts[state as usize] = 0;
        }
    }

    normalize(blocks, ranges.len())
}

fn normalize(mut blocks: Vec<u32>, classes: usize) -> Vec<u32> {
    let mut mapping = vec![u32::MAX; classes];
    let mut next_id = 0;
    for block in &mut blocks {
        let entry = &mut mapping[*block as usize];
        if *entry == u32::MAX {
            *entry = next_id;
            next_id += 1;
        }
        *block = *entry;
    }
    blocks
}

fn exact_stable(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    ranges: &[(u32, u32)],
    buffer: &[u32],
) -> bool {
    let mut counts = vec![0_u32; ranges.len()];
    for &(start, end) in ranges {
        if end - start <= 1 {
            continue;
        }
        let representative = buffer[start as usize];
        graph.visit_successors(representative, |target| {
            counts[blocks[target as usize] as usize] += 1;
        });
        for &state in &buffer[start as usize + 1..end as usize] {
            let mut equal = true;
            graph.visit_successors(state, |target| {
                let count = &mut counts[blocks[target as usize] as usize];
                if *count == 0 {
                    equal = false;
                } else {
                    *count -= 1;
                }
            });
            if !equal {
                return false;
            }
            graph.visit_successors(state, |target| {
                counts[blocks[target as usize] as usize] += 1;
            });
        }
        graph.visit_successors(representative, |target| {
            counts[blocks[target as usize] as usize] = 0;
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{refine, refine_hybrid, refine_impl, CountRefinementGraph};
    use std::collections::HashMap;

    struct DirectedGraph {
        tags: Vec<u32>,
        successors: Vec<Vec<u32>>,
        predecessors: Vec<Vec<u32>>,
    }

    impl DirectedGraph {
        fn new(tags: Vec<u32>, successors: Vec<Vec<u32>>) -> Self {
            let mut predecessors = vec![Vec::new(); tags.len()];
            for (source, targets) in successors.iter().enumerate() {
                for &target in targets {
                    predecessors[target as usize].push(source as u32);
                }
            }
            Self {
                tags,
                successors,
                predecessors,
            }
        }
    }

    impl CountRefinementGraph for DirectedGraph {
        fn num_states(&self) -> usize {
            self.tags.len()
        }

        fn initial_signature(&self, state: u32) -> (u32, u32) {
            (
                self.tags[state as usize],
                self.successors[state as usize].len() as u32,
            )
        }

        fn visit_predecessors(&self, state: u32, mut visit: impl FnMut(u32)) {
            for &source in &self.predecessors[state as usize] {
                visit(source);
            }
        }

        fn visit_successors(&self, state: u32, mut visit: impl FnMut(u32)) {
            for &target in &self.successors[state as usize] {
                visit(target);
            }
        }
    }

    fn exact_reference(graph: &DirectedGraph) -> Vec<u32> {
        let n = graph.num_states();
        let mut initial_ids = HashMap::new();
        let mut blocks = Vec::with_capacity(n);
        for state in 0..n as u32 {
            let next_id = initial_ids.len() as u32;
            blocks.push(
                *initial_ids
                    .entry(graph.initial_signature(state))
                    .or_insert(next_id),
            );
        }
        loop {
            let mut signature_ids = HashMap::new();
            let mut next_blocks = Vec::with_capacity(n);
            for state in 0..n {
                let mut targets = graph.successors[state]
                    .iter()
                    .map(|&target| blocks[target as usize])
                    .collect::<Vec<_>>();
                targets.sort_unstable();
                let next_id = signature_ids.len() as u32;
                next_blocks.push(
                    *signature_ids
                        .entry((blocks[state], targets))
                        .or_insert(next_id),
                );
            }
            if next_blocks == blocks {
                return blocks;
            }
            blocks = next_blocks;
        }
    }

    struct Rng(u64);

    impl Rng {
        fn index(&mut self, bound: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 as usize) % bound
        }
    }

    fn check(graph: &DirectedGraph, case: usize) {
        let expected = exact_reference(graph);
        assert_eq!(refine(graph), expected, "incremental case {case}");
        assert_eq!(
            refine_impl(graph, true, super::mix, 512),
            expected,
            "hybrid case {case}"
        );
        assert_eq!(
            refine_impl(graph, true, |_| 0, 512),
            expected,
            "colliding seed case {case}"
        );
    }

    #[test]
    fn mixed_tags_preserve_degree_sixty_four_boundary() {
        let signatures = [
            (0, 1),
            (7, 65),
            (8, 64),
            (7, 64),
            (9, 64),
            (8, 64),
            (7, 65),
            (8, 65),
            (8, 0),
            (9, 0),
            (7, 64),
            (0, 1),
        ];
        for rotation in 0..signatures.len() {
            let signatures = signatures[rotation..].iter().chain(&signatures[..rotation]);
            let (tags, successors) = signatures
                .enumerate()
                .map(|(state, &(tag, degree))| (tag, vec![state as u32; degree]))
                .unzip();
            check(&DirectedGraph::new(tags, successors), rotation);
        }
    }

    #[test]
    fn sparse_directed_hybrid_and_colliding_seed_match_exact_reference() {
        let mut rng = Rng(0x8d91_b2e4_72a6_930f);
        for case in 0..12 {
            let n = 512 + case * 9;
            let tags = (0..n).map(|_| rng.index(3) as u32).collect();
            let successors = (0..n)
                .map(|state| {
                    let mut targets = Vec::new();
                    for _ in 0..rng.index(9) {
                        let target = rng.index(n) as u32;
                        targets.push(target);
                        if rng.index(4) == 0 {
                            targets.push(target);
                        }
                    }
                    if state % 5 == 0 {
                        targets.extend([state as u32, state as u32]);
                    }
                    targets
                })
                .collect();
            check(&DirectedGraph::new(tags, successors), case);
        }
    }

    #[test]
    fn sparse_directed_hybrid_preserves_nonsingleton_classes() {
        let mut rng = Rng(0x91c3_a67f_043b_25e8);
        let n = 576;
        let mut tags = vec![0; n];
        let mut successors = vec![Vec::new(); n];
        for group in 0..n / 3 {
            let tag = rng.index(3) as u32;
            let mut targets = Vec::new();
            for _ in 0..rng.index(7) {
                let target_group = rng.index(n / 3);
                for target in 0..3 {
                    targets.push((target_group * 3 + target) as u32);
                }
            }
            if group % 7 == 0 {
                for target in 0..3 {
                    targets.extend([(group * 3 + target) as u32; 2]);
                }
            }
            for member in 0..3 {
                tags[group * 3 + member] = tag;
                successors[group * 3 + member] = targets.clone();
            }
        }
        let graph = DirectedGraph::new(tags, successors);
        check(&graph, 0);
        let actual = refine_hybrid(&graph);
        for group in 0..n / 3 {
            assert_eq!(actual[group * 3], actual[group * 3 + 1]);
            assert_eq!(actual[group * 3], actual[group * 3 + 2]);
        }
    }

    #[test]
    fn marking_a_self_splitter_preserves_targets_and_multiplicity() {
        let graph = DirectedGraph::new(
            vec![0, 0, 0, 0, 1, 1],
            vec![
                vec![0, 4, 4],
                vec![1, 5, 5],
                vec![2, 0, 4],
                vec![3, 1, 4],
                vec![4, 0, 0],
                vec![5, 2, 3],
            ],
        );
        check(&graph, 0);
    }

    #[test]
    fn directed_tail_marking_matches_exact_reference() {
        let mut rng = Rng(0x286e_5c17_d4b0_03a9);
        for case in 0..256 {
            let n = 3 + rng.index(46);
            let tags = (0..n).map(|_| rng.index(4) as u32).collect();
            let successors = (0..n)
                .map(|state| {
                    let mut targets = (0..rng.index(24))
                        .map(|_| rng.index(n) as u32)
                        .collect::<Vec<_>>();
                    if state % 3 == 0 {
                        targets.extend([state as u32; 3]);
                    }
                    targets
                })
                .collect();
            check(&DirectedGraph::new(tags, successors), case);
        }
    }
}

pub(crate) fn exact_small_stable(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    classes: usize,
) -> bool {
    let mut expected = [[0_u32; 4]; 4];
    let mut initialized = [false; 4];
    for state in 0..blocks.len() as u32 {
        let mut signature = [0_u32; 4];
        graph.visit_successors(state, |target| {
            signature[blocks[target as usize] as usize] += 1;
        });
        let block = blocks[state as usize] as usize;
        debug_assert!(block < classes);
        if initialized[block] {
            if expected[block] != signature {
                return false;
            }
        } else {
            expected[block] = signature;
            initialized[block] = true;
        }
    }
    true
}
