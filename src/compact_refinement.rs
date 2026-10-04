use crate::count_refinement::CountRefinementGraph;
use fxhash::FxHashMap;

const SINGLETON: u32 = u32::MAX;

struct State {
    cell: u32,
    count: u32,
    position: u32,
}

#[derive(Clone, Copy)]
struct Cell {
    size: u32,
    hits: u32,
}

#[cfg(test)]
fn refine(graph: &impl CountRefinementGraph) -> Option<Vec<u32>> {
    refine_impl(graph, false)
}

pub(crate) fn try_refine(graph: &impl CountRefinementGraph) -> Option<Vec<u32>> {
    refine_impl(graph, true)
}

fn refine_impl(graph: &impl CountRefinementGraph, automatic: bool) -> Option<Vec<u32>> {
    let n = graph.num_states();
    if n > 16_384 {
        return None;
    }
    if n == 0 {
        return Some(Vec::new());
    }
    let first = graph.initial_signature(0);
    let mut prefix = 1;
    while prefix < n && graph.initial_signature(prefix as u32) == first {
        prefix += 1;
    }
    if prefix == n {
        return Some(vec![0; n]);
    }
    let mut degree_ids = vec![u32::MAX; first.1 as usize + 1];
    degree_ids[first.1 as usize] = 0;
    let mut other_ids = FxHashMap::default();
    let mut sizes = vec![prefix as u32];
    let mut blocks = Vec::with_capacity(n);
    blocks.resize(prefix, 0_u32);
    let mut max_degree = first.1;
    let mut total_degree = first.1 as u64 * prefix as u64;
    for state in prefix..n {
        let signature = graph.initial_signature(state as u32);
        max_degree = max_degree.max(signature.1);
        total_degree += signature.1 as u64;
        let next = sizes.len() as u32;
        let block = if signature.0 == first.0 {
            degree_ids.resize(degree_ids.len().max(signature.1 as usize + 1), u32::MAX);
            let id = &mut degree_ids[signature.1 as usize];
            if *id == u32::MAX {
                *id = next;
                sizes.push(0);
            }
            *id
        } else {
            *other_ids.entry(signature).or_insert_with(|| {
                sizes.push(0);
                next
            })
        };
        sizes[block as usize] += 1;
        blocks.push(block);
    }
    if automatic
        && max_degree > 64
        && (degree_ids
            .iter()
            .enumerate()
            .any(|(degree, &block)| degree > 64 && block != u32::MAX && sizes[block as usize] > 1)
            || other_ids
                .iter()
                .any(|(&(_, degree), &block)| degree > 64 && sizes[block as usize] > 1))
    {
        return None;
    }
    if sizes.len() == n || max_degree == 0 {
        return Some(blocks);
    }

    if sizes.len() <= 4
        && total_degree <= n as u64 * 8
        && crate::count_refinement::exact_small_stable(graph, &blocks, sizes.len())
    {
        return Some(blocks);
    }

    let largest = sizes
        .iter()
        .enumerate()
        .max_by_key(|&(_, size)| size)
        .unwrap()
        .0;
    let mut cells = vec![Cell { size: 0, hits: 0 }; n];
    let mut starts = Vec::with_capacity(sizes.len());
    let mut cursor = 0_u32;
    for &size in &sizes {
        starts.push(cursor);
        cells[cursor as usize].size = size;
        cursor += size;
    }
    let mut insertion = starts.clone();
    let mut members = vec![0_u32; n];
    let mut states = blocks
        .into_iter()
        .enumerate()
        .map(|(state, block)| {
            let block = block as usize;
            let slot = &mut insertion[block];
            members[*slot as usize] = state as u32;
            let record = State {
                cell: if sizes[block] == 1 {
                    SINGLETON
                } else {
                    starts[block]
                },
                count: 0,
                position: *slot,
            };
            *slot += 1;
            record
        })
        .collect::<Vec<_>>();
    let mut worklist = starts
        .iter()
        .enumerate()
        .filter(|&(block, _)| block != largest)
        .map(|(_, &start)| start)
        .collect::<Vec<_>>();
    worklist.sort_unstable_by_key(|&cell| std::cmp::Reverse(cells[cell as usize].size));
    let mut class_count = sizes.len();
    let mut hit_cells = Vec::new();
    let mut hit_members = vec![0_u32; n];
    let mut frequencies = vec![0_u32; max_degree as usize + 1];
    let mut destinations = vec![0_u32; max_degree as usize + 1];
    let mut used_counts = Vec::with_capacity((max_degree as usize).min(n));
    let mut fragments = Vec::with_capacity((max_degree as usize + 1).min(n));

    while !worklist.is_empty() && class_count < n {
        let first = worklist.len().saturating_sub(8);
        let selected = (first..worklist.len())
            .min_by_key(|&index| cells[worklist[index] as usize].size)
            .unwrap();
        let splitter = worklist.swap_remove(selected) as usize;
        hit_cells.clear();
        for &target in &members[splitter..splitter + cells[splitter].size as usize] {
            graph.visit_predecessors(target, |state| {
                let record = &mut states[state as usize];
                if record.cell == SINGLETON {
                    return;
                }
                if record.count == 0 {
                    let cell = &mut cells[record.cell as usize];
                    if cell.hits == 0 {
                        hit_cells.push(record.cell as usize);
                    }
                    hit_members[record.cell as usize + cell.hits as usize] = state;
                    cell.hits += 1;
                }
                record.count += 1;
            });
        }

        for &cell_index in &hit_cells {
            let cell = cells[cell_index];
            cells[cell_index].hits = 0;
            let zero_size = cell.size - cell.hits;
            let hits = &hit_members[cell_index..cell_index + cell.hits as usize];
            let first_count = states[hits[0] as usize].count;
            let mut prefix = 0;
            while prefix < hits.len() && states[hits[prefix] as usize].count == first_count {
                states[hits[prefix] as usize].count = 0;
                prefix += 1;
            }
            let uniform = prefix == hits.len();
            if !uniform {
                // A failed uniform scan must restore the counts used by the histogram.
                for &state in &hits[..prefix] {
                    states[state as usize].count = first_count;
                }
            }
            if uniform && zero_size == 0 {
                continue;
            }

            if uniform {
                let (kept_size, moved_size, destination) = if zero_size >= cell.hits {
                    (zero_size, cell.hits, cell_index as u32 + zero_size)
                } else {
                    (cell.hits, zero_size, cell_index as u32)
                };
                let mut destination = destination;
                for &state in hits {
                    move_state(&mut states, &mut members, state, destination);
                    destination += 1;
                }
                cells[cell_index].size = kept_size;
                if kept_size == 1 {
                    states[members[cell_index] as usize].cell = SINGLETON;
                }
                let moved = cell_index + kept_size as usize;
                cells[moved].size = moved_size;
                let id = if moved_size == 1 {
                    SINGLETON
                } else {
                    moved as u32
                };
                for &state in &members[moved..moved + moved_size as usize] {
                    states[state as usize].cell = id;
                }
                class_count += 1;
                worklist.push(moved as u32);
                continue;
            }

            used_counts.clear();
            for &state in hits {
                let count = states[state as usize].count as usize;
                if frequencies[count] == 0 {
                    used_counts.push(count);
                }
                frequencies[count] += 1;
            }
            let mut largest_count = 0;
            let mut largest_size = zero_size;
            for &count in &used_counts {
                if frequencies[count] > largest_size {
                    largest_count = count;
                    largest_size = frequencies[count];
                }
            }
            fragments.clear();
            let mut cursor = cell_index as u32 + largest_size;
            if largest_count != 0 {
                destinations[largest_count] = cell_index as u32;
                if zero_size != 0 {
                    fragments.push((cursor, zero_size));
                    cursor += zero_size;
                }
            }
            for &count in &used_counts {
                if count != largest_count {
                    let size = frequencies[count];
                    destinations[count] = cursor;
                    fragments.push((cursor, size));
                    cursor += size;
                }
                frequencies[count] = 0;
            }

            // Only touched states move; the largest fragment retains this cell's start.
            for &state in hits {
                let destination = &mut destinations[states[state as usize].count as usize];
                move_state(&mut states, &mut members, state, *destination);
                states[state as usize].count = 0;
                *destination += 1;
            }
            cells[cell_index].size = largest_size;
            if largest_size == 1 {
                states[members[cell_index] as usize].cell = SINGLETON;
            }
            for &(start, size) in &fragments {
                let start = start as usize;
                cells[start].size = size;
                let id = if size == 1 { SINGLETON } else { start as u32 };
                for &state in &members[start..start + size as usize] {
                    states[state as usize].cell = id;
                }
                class_count += 1;
                worklist.push(start as u32);
            }
        }
    }
    if class_count == n {
        for (state, member) in members.iter_mut().enumerate() {
            *member = state as u32;
        }
        return Some(members);
    }
    let mut next_id = 0;
    for (member, state) in members.iter_mut().zip(states) {
        *member = if state.cell == SINGLETON {
            let id = next_id;
            next_id += 1;
            id
        } else {
            let id = &mut cells[state.cell as usize].hits;
            if *id == 0 {
                next_id += 1;
                *id = next_id;
            }
            *id - 1
        };
    }
    Some(members)
}

#[inline]
fn move_state(states: &mut [State], members: &mut [u32], state: u32, destination: u32) {
    let previous = states[state as usize].position;
    let other = members[destination as usize];
    members[previous as usize] = other;
    states[other as usize].position = previous;
    members[destination as usize] = state;
    states[state as usize].position = destination;
}

#[cfg(test)]
mod tests {
    use super::{refine, try_refine};
    use crate::count_refinement::CountRefinementGraph;
    use std::collections::HashMap;

    struct Graph {
        tags: Vec<u32>,
        successors: Vec<Vec<u32>>,
        predecessors: Vec<Vec<u32>>,
    }

    impl Graph {
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

    impl CountRefinementGraph for Graph {
        fn num_states(&self) -> usize {
            self.tags.len()
        }
        fn initial_signature(&self, state: u32) -> (u32, u32) {
            (
                self.tags[state as usize],
                self.successors[state as usize].len() as u32,
            )
        }
        fn visit_successors(&self, state: u32, mut visit: impl FnMut(u32)) {
            for &target in &self.successors[state as usize] {
                visit(target);
            }
        }
        fn visit_predecessors(&self, state: u32, mut visit: impl FnMut(u32)) {
            for &source in &self.predecessors[state as usize] {
                visit(source);
            }
        }
    }

    #[test]
    fn automatic_refinement_matches_exact_small_and_medium_directed_graphs() {
        let mut seed = 0x9e87_ba65_4132_df01_u64;
        let mut next = |bound: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as usize % bound
        };
        for case in 0..64 {
            let n = 1 + next(320);
            let tags = (0..n).map(|_| next(3) as u32).collect();
            let successors = (0..n)
                .map(|_| (0..next(9)).map(|_| next(n) as u32).collect())
                .collect();
            let graph = Graph::new(tags, successors);
            let expected = reference(&graph);
            assert_eq!(try_refine(&graph), Some(expected.clone()), "case {case}");
            assert_eq!(crate::count_refinement::refine_hybrid(&graph), expected);
        }
    }

    #[test]
    fn high_degree_singletons_do_not_reject_compact_refinement() {
        let mut successors = vec![vec![0]; 66];
        successors[0] = (0..66).collect();
        let graph = Graph::new(vec![0; 66], successors);
        assert_eq!(try_refine(&graph), Some(reference(&graph)));
    }

    fn reference(graph: &Graph) -> Vec<u32> {
        let mut ids = HashMap::new();
        let mut blocks = (0..graph.num_states() as u32)
            .map(|state| {
                let next = ids.len() as u32;
                *ids.entry(graph.initial_signature(state)).or_insert(next)
            })
            .collect::<Vec<_>>();
        loop {
            let mut ids = HashMap::new();
            let next_blocks = (0..blocks.len())
                .map(|state| {
                    let mut targets = graph.successors[state]
                        .iter()
                        .map(|&target| blocks[target as usize])
                        .collect::<Vec<_>>();
                    targets.sort_unstable();
                    let next = ids.len() as u32;
                    *ids.entry((blocks[state], targets)).or_insert(next)
                })
                .collect::<Vec<_>>();
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
            self.0 as usize % bound
        }
    }

    #[test]
    fn directed_multigraphs_match_exact_signatures() {
        let mut rng = Rng(0x773b_39c8_ba4d_56d1);
        for case in 0..512 {
            let n = 1 + rng.index(90);
            let tags = (0..n).map(|_| rng.index(4) as u32).collect();
            let successors = (0..n)
                .map(|state| {
                    let mut targets = (0..rng.index(25))
                        .map(|_| rng.index(n) as u32)
                        .collect::<Vec<_>>();
                    if state % 3 == 0 {
                        targets.extend([state as u32; 3]);
                    }
                    targets
                })
                .collect();
            let graph = Graph::new(tags, successors);
            let expected = reference(&graph);
            assert_eq!(refine(&graph), Some(expected.clone()), "case {case}");
        }
    }

    #[test]
    fn queued_cell_splits_and_singleton_splitters_are_exact() {
        let graph = Graph::new(
            vec![0, 0, 0, 0, 1, 1, 2],
            vec![
                vec![0, 4, 4],
                vec![1, 5, 5],
                vec![2, 0, 4],
                vec![3, 1, 4],
                vec![4, 0, 6],
                vec![5, 2, 3],
                vec![5],
            ],
        );
        assert_eq!(refine(&graph), Some(reference(&graph)));
    }

    #[test]
    fn nondiscrete_medium_graph_preserves_equivalent_members() {
        let mut rng = Rng(0x679e_281d_61ba_abc3);
        let groups = 128;
        let mut tags = vec![0; groups * 3];
        let mut successors = vec![Vec::new(); groups * 3];
        for group in 0..groups {
            let tag = rng.index(3) as u32;
            let mut targets = Vec::new();
            for _ in 0..4 + rng.index(3) {
                let target = rng.index(groups);
                targets.extend((target * 3..target * 3 + 3).map(|v| v as u32));
            }
            for member in 0..3 {
                tags[group * 3 + member] = tag;
                successors[group * 3 + member] = targets.clone();
            }
        }
        let graph = Graph::new(tags, successors);
        assert_eq!(refine(&graph), Some(reference(&graph)));
    }

    #[test]
    fn supports_histogram_counts_above_sixty_four() {
        let mut first = vec![2; 64];
        first.push(0);
        let mut second = vec![2; 63];
        second.extend([1; 2]);
        let graph = Graph::new(vec![0, 0, 1], vec![first, second, vec![0]]);
        assert_eq!(refine(&graph), Some(reference(&graph)));
    }

    #[test]
    fn stable_initial_cells_preserve_equivalent_members() {
        let graph = Graph::new(
            vec![0, 0, 1, 1],
            vec![vec![0, 2, 2], vec![1, 3, 3], vec![0], vec![1]],
        );
        assert_eq!(refine(&graph), Some(vec![0, 0, 1, 1]));
    }

    #[test]
    fn singleton_sentinel_preserves_distinct_singletons_in_normalization() {
        let graph = Graph::new(
            vec![0, 0, 1, 2],
            vec![vec![0, 1], vec![0, 1], vec![0], vec![1]],
        );
        assert_eq!(refine(&graph), Some(vec![0, 0, 1, 2]));
    }

    #[test]
    fn nonuniform_counts_restore_the_cleared_prefix() {
        let graph = Graph::new(
            vec![0, 0, 0, 0, 0, 0, 1, 1],
            vec![
                vec![6, 0, 0],
                vec![6, 1, 1],
                vec![6, 6, 2],
                vec![7, 3, 3],
                vec![7, 7, 4],
                vec![5, 5, 5],
                vec![0],
                vec![1],
            ],
        );
        let expected = vec![0, 0, 1, 0, 1, 2, 3, 3];
        assert_eq!(reference(&graph), expected);
        assert_eq!(refine(&graph), Some(expected.clone()));
    }

    #[test]
    fn fully_hit_uniform_cells_clear_counts_before_the_next_splitter() {
        let graph = Graph::new(
            vec![0, 0, 0, 1, 1, 1, 2, 2, 3, 4],
            vec![
                vec![6, 0, 0, 3],
                vec![7, 1, 1, 4],
                vec![6, 2, 2, 5],
                vec![6, 0, 0, 0],
                vec![7, 1, 4, 4],
                vec![6, 2, 5, 5],
                vec![6, 0, 3],
                vec![7, 1, 4],
                vec![],
                vec![],
            ],
        );
        let expected = reference(&graph);
        assert_ne!(expected[3], expected[4]);
        assert_eq!(refine(&graph), Some(expected.clone()));
    }
}
