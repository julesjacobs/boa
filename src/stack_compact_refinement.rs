use crate::count_refinement::CountRefinementGraph;
use fxhash::FxHashMap;

const SINGLETON: u32 = u32::MAX;
pub(crate) const MAX_STATES: usize = 2048;

#[derive(Clone, Copy)]
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

pub(crate) fn refine(graph: &impl CountRefinementGraph) -> Option<Vec<u32>> {
    let n = graph.num_states();
    if n > MAX_STATES {
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
    if first.1 > 64 && prefix > 1 {
        return None;
    }
    match n {
        1..=64 => refine_capacity::<64>(graph, first, prefix),
        65..=128 => refine_capacity::<128>(graph, first, prefix),
        129..=256 => refine_capacity::<256>(graph, first, prefix),
        257..=512 => refine_capacity::<512>(graph, first, prefix),
        513..=768 => refine_capacity::<768>(graph, first, prefix),
        769..=1024 => refine_capacity::<1024>(graph, first, prefix),
        1025..=1536 => refine_capacity::<1536>(graph, first, prefix),
        _ => refine_capacity::<2048>(graph, first, prefix),
    }
}

#[inline(never)]
fn refine_capacity<const C: usize>(
    graph: &impl CountRefinementGraph,
    first: (u32, u32),
    prefix: usize,
) -> Option<Vec<u32>> {
    let mut states = [State {
        cell: 0,
        count: 0,
        position: 0,
    }; C];
    let mut cells = [Cell { size: 0, hits: 0 }; C];
    let mut sizes = [0_u32; C];
    let mut worklist = [0_u32; C];
    let mut hit_cells = [0_usize; C];
    let mut hit_members = [0_u32; C];
    refine_with_workspace(
        graph,
        first,
        prefix,
        &mut states,
        &mut cells,
        &mut sizes,
        &mut worklist,
        &mut hit_cells,
        &mut hit_members,
    )
}

#[inline(never)]
fn refine_with_workspace(
    graph: &impl CountRefinementGraph,
    first: (u32, u32),
    prefix: usize,
    states: &mut [State],
    cells: &mut [Cell],
    sizes: &mut [u32],
    worklist: &mut [u32],
    hit_cells: &mut [usize],
    hit_members: &mut [u32],
) -> Option<Vec<u32>> {
    let n = graph.num_states();
    let mut degree_ids = [u32::MAX; 65];
    let mut other_ids = FxHashMap::default();
    if first.1 <= 64 {
        degree_ids[first.1 as usize] = 0;
    } else {
        other_ids.insert(first, 0);
    }
    sizes[0] = prefix as u32;
    let mut class_count = 1;
    let mut members = Vec::with_capacity(n);
    members.resize(prefix, 0_u32);
    let mut total_degree = first.1 as u64 * prefix as u64;
    for state in prefix..n {
        let signature = graph.initial_signature(state as u32);
        total_degree += signature.1 as u64;
        let next = class_count as u32;
        let block = if signature.0 == first.0 && signature.1 <= 64 {
            let id = &mut degree_ids[signature.1 as usize];
            if *id == u32::MAX {
                *id = next;
                class_count += 1;
            }
            *id
        } else {
            *other_ids.entry(signature).or_insert_with(|| {
                class_count += 1;
                next
            })
        };
        sizes[block as usize] += 1;
        if signature.1 > 64 && sizes[block as usize] > 1 {
            return None;
        }
        members.push(block);
        states[state].cell = block;
    }
    if class_count == n || total_degree == 0 {
        return Some(members);
    }

    if class_count <= 4
        && total_degree <= n as u64 * 8
        && crate::count_refinement::exact_small_stable(graph, &members, class_count)
    {
        return Some(members);
    }

    let largest = sizes[..class_count]
        .iter()
        .enumerate()
        .max_by_key(|&(_, size)| size)
        .unwrap()
        .0;
    // Splitter buffers hold starts and insertion cursors during initialization.
    let mut cursor = 0_u32;
    for (class, &size) in sizes[..class_count].iter().enumerate() {
        hit_members[class] = cursor;
        worklist[class] = cursor;
        cells[cursor as usize].size = size;
        cursor += size;
    }
    for (state, record) in states[..n].iter_mut().enumerate() {
        let block = record.cell as usize;
        let slot = &mut worklist[block];
        members[*slot as usize] = state as u32;
        *record = State {
            cell: if sizes[block] == 1 {
                SINGLETON
            } else {
                hit_members[block]
            },
            count: 0,
            position: *slot,
        };
        *slot += 1;
    }
    let mut pending = 0;
    for (block, &start) in hit_members[..class_count].iter().enumerate() {
        if block != largest {
            worklist[pending] = start;
            pending += 1;
        }
    }
    worklist[..pending].sort_unstable_by_key(|&cell| std::cmp::Reverse(cells[cell as usize].size));
    let mut frequencies = [0_u32; 65];
    let mut destinations = [0_u32; 65];
    let mut used_counts = [0_usize; 65];
    let mut fragments = [(0_u32, 0_u32); 65];

    while pending != 0 && class_count < n {
        let first = pending.saturating_sub(8);
        let selected = (first..pending)
            .min_by_key(|&index| cells[worklist[index] as usize].size)
            .unwrap();
        let splitter = worklist[selected] as usize;
        pending -= 1;
        worklist[selected] = worklist[pending];
        let mut hit_count = 0;
        for &target in &members[splitter..splitter + cells[splitter].size as usize] {
            graph.visit_predecessors(target, |state| {
                let record = &mut states[state as usize];
                if record.cell == SINGLETON {
                    return;
                }
                if record.count == 0 {
                    let cell = &mut cells[record.cell as usize];
                    if cell.hits == 0 {
                        hit_cells[hit_count] = record.cell as usize;
                        hit_count += 1;
                    }
                    hit_members[record.cell as usize + cell.hits as usize] = state;
                    cell.hits += 1;
                }
                record.count += 1;
            });
        }

        for &cell_index in &hit_cells[..hit_count] {
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
                    move_state(states, &mut members, state, destination);
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
                worklist[pending] = moved as u32;
                pending += 1;
                continue;
            }

            let mut used_count = 0;
            for &state in hits {
                let count = states[state as usize].count as usize;
                if frequencies[count] == 0 {
                    used_counts[used_count] = count;
                    used_count += 1;
                }
                frequencies[count] += 1;
            }
            let mut largest_count = 0;
            let mut largest_size = zero_size;
            for &count in &used_counts[..used_count] {
                if frequencies[count] > largest_size {
                    largest_count = count;
                    largest_size = frequencies[count];
                }
            }
            let mut fragment_count = 0;
            let mut cursor = cell_index as u32 + largest_size;
            if largest_count != 0 {
                destinations[largest_count] = cell_index as u32;
                if zero_size != 0 {
                    fragments[fragment_count] = (cursor, zero_size);
                    fragment_count += 1;
                    cursor += zero_size;
                }
            }
            for &count in &used_counts[..used_count] {
                if count != largest_count {
                    let size = frequencies[count];
                    destinations[count] = cursor;
                    fragments[fragment_count] = (cursor, size);
                    fragment_count += 1;
                    cursor += size;
                }
                frequencies[count] = 0;
            }

            // Only touched states move; the largest fragment retains this cell's start.
            for &state in hits {
                let destination = &mut destinations[states[state as usize].count as usize];
                move_state(states, &mut members, state, *destination);
                states[state as usize].count = 0;
                *destination += 1;
            }
            cells[cell_index].size = largest_size;
            if largest_size == 1 {
                states[members[cell_index] as usize].cell = SINGLETON;
            }
            for &(start, size) in &fragments[..fragment_count] {
                let start = start as usize;
                cells[start].size = size;
                let id = if size == 1 { SINGLETON } else { start as u32 };
                for &state in &members[start..start + size as usize] {
                    states[state as usize].cell = id;
                }
                class_count += 1;
                worklist[pending] = start as u32;
                pending += 1;
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
    for (member, state) in members.iter_mut().zip(&states[..n]) {
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
    use super::refine;
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

    fn reference(graph: &Graph) -> Vec<u32> {
        let mut ids = HashMap::new();
        let mut classes = (0..graph.num_states() as u32)
            .map(|state| {
                let next = ids.len() as u32;
                *ids.entry(graph.initial_signature(state)).or_insert(next)
            })
            .collect::<Vec<_>>();
        loop {
            let mut ids = HashMap::new();
            let next_classes = (0..classes.len())
                .map(|state| {
                    let mut targets = graph.successors[state]
                        .iter()
                        .map(|&target| classes[target as usize])
                        .collect::<Vec<_>>();
                    targets.sort_unstable();
                    let next = ids.len() as u32;
                    *ids.entry((classes[state], targets)).or_insert(next)
                })
                .collect::<Vec<_>>();
            if next_classes == classes {
                return classes;
            }
            classes = next_classes;
        }
    }

    fn check(graph: &Graph) {
        let expected = reference(graph);
        assert_eq!(refine(graph), Some(expected.clone()));
        assert_eq!(crate::compact_refinement::try_refine(graph), Some(expected));
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
    fn capacity_boundaries_preserve_exact_directed_multigraphs() {
        let mut rng = Rng(0x673d_018a_ab69_402e);
        for n in [
            1, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513, 767, 768, 769, 1023, 1024,
            1025, 1535, 1536, 1537, 2047, 2048,
        ] {
            for _ in 0..8 {
                let tags = (0..n).map(|_| rng.index(4) as u32).collect();
                let successors = (0..n)
                    .map(|state| {
                        let mut targets = (0..rng.index(22))
                            .map(|_| rng.index(n) as u32)
                            .collect::<Vec<_>>();
                        if state % 3 == 0 {
                            targets.extend([state as u32; 3]);
                        }
                        targets
                    })
                    .collect();
                check(&Graph::new(tags, successors));
            }
        }
    }

    #[test]
    fn nonuniform_counts_restore_the_cleared_prefix() {
        check(&Graph::new(
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
        ));
    }

    #[test]
    fn fully_hit_uniform_cells_clear_counts_before_the_next_splitter() {
        check(&Graph::new(
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
        ));
    }

    #[test]
    fn nondiscrete_partition_preserves_equivalent_members() {
        let mut rng = Rng(0x204a_c3f5_158e_09b7);
        let n = 512;
        let mut tags = vec![0; n];
        let mut successors = vec![Vec::new(); n];
        for group in 0..n / 4 {
            let tag = rng.index(3) as u32;
            let mut targets = Vec::new();
            for _ in 0..4 + rng.index(3) {
                let target_group = rng.index(n / 4);
                targets.extend((target_group * 4..target_group * 4 + 4).map(|v| v as u32));
            }
            for member in 0..4 {
                tags[group * 4 + member] = tag;
                successors[group * 4 + member] = targets.clone();
            }
        }
        let graph = Graph::new(tags, successors);
        let expected = reference(&graph);
        for members in expected.chunks_exact(4) {
            assert!(members.iter().all(|&class| class == members[0]));
        }
        assert_eq!(refine(&graph), Some(expected));
    }

    #[test]
    fn count_sixty_four_and_singleton_normalization_are_exact() {
        check(&Graph::new(
            vec![0, 0, 1],
            vec![vec![2; 64], vec![1; 64], vec![0]],
        ));
        check(&Graph::new(
            vec![0, 0, 1, 2, 3, 4],
            vec![vec![0, 1], vec![0, 1], vec![0], vec![1], vec![], vec![]],
        ));
    }

    #[test]
    fn first_and_last_high_degree_singletons_keep_count_sixty_four_exact() {
        let n = 70;
        for hub in [0, n - 1] {
            let successors = (0..n)
                .map(|state| {
                    if state == hub {
                        vec![hub as u32; 65]
                    } else {
                        match state % 3 {
                            0 => vec![hub as u32; 64],
                            1 => {
                                let mut targets = vec![hub as u32; 63];
                                targets.push(state as u32);
                                targets
                            }
                            _ => vec![state as u32; 64],
                        }
                    }
                })
                .collect();
            check(&Graph::new(vec![0; n], successors));
        }
    }

    #[test]
    fn high_degree_singleton_signatures_preserve_tags_and_degrees() {
        for (second_tag, second_degree) in [(8, 65), (7, 66)] {
            check(&Graph::new(
                vec![7, second_tag, 7, 7, 8, 8],
                vec![
                    vec![0; 65],
                    vec![1; second_degree],
                    vec![0; 64],
                    vec![3; 64],
                    vec![1; 64],
                    vec![5; 64],
                ],
            ));
        }
    }

    #[test]
    fn high_degree_nonsingleton_initial_classes_use_fallback() {
        for graph in [
            Graph::new(vec![0, 0, 1], vec![vec![0; 65], vec![1; 65], vec![2]]),
            Graph::new(vec![0, 1, 0], vec![vec![0; 65], vec![1], vec![2; 65]]),
            Graph::new(vec![0, 1, 1], vec![vec![0], vec![1; 65], vec![2; 65]]),
        ] {
            assert_eq!(refine(&graph), None);
            assert_eq!(
                crate::count_refinement::refine_hybrid(&graph),
                reference(&graph)
            );
        }
    }

    #[test]
    fn empty_graph_and_fallback_limits() {
        check(&Graph::new(vec![], vec![]));
        let graph = Graph::new(vec![0; 2049], vec![vec![]; 2049]);
        assert_eq!(refine(&graph), None);
    }
}
