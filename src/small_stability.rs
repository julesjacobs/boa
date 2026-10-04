use crate::count_refinement::CountRefinementGraph;

/// Certifies an initial partition whose cells have fixed tags and outdegrees.
pub(crate) fn small_initial_stable(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    sizes: &[u32],
    max_degree: u32,
) -> bool {
    certify(graph, blocks, sizes, max_degree, false)
}

pub(crate) fn small_initial_stable_by_volume(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    sizes: &[u32],
    max_degree: u32,
) -> bool {
    certify(graph, blocks, sizes, max_degree, true)
}

fn certify(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    sizes: &[u32],
    max_degree: u32,
    by_volume: bool,
) -> bool {
    debug_assert_eq!(blocks.len(), graph.num_states());
    if sizes.len() > 4 {
        return false;
    }
    if sizes.len() <= 1 {
        return true;
    }

    let omitted = if by_volume {
        let mut class_degrees = [0_u32; 4];
        let mut seen = [false; 4];
        let mut found = 0;
        for (state, &block) in blocks.iter().enumerate() {
            let block = block as usize;
            if !seen[block] {
                class_degrees[block] = graph.initial_signature(state as u32).1;
                seen[block] = true;
                found += 1;
                if found == sizes.len() {
                    break;
                }
            }
        }
        (0..sizes.len())
            .max_by_key(|&block| sizes[block] as u64 * class_degrees[block] as u64)
            .unwrap()
    } else {
        sizes
            .iter()
            .enumerate()
            .max_by_key(|&(_, size)| size)
            .unwrap()
            .0
    };
    let columns = sizes.len() - 1;
    let mut column_ids = [usize::MAX; 4];
    let mut next_column = 0;
    for (block, column) in column_ids[..sizes.len()].iter_mut().enumerate() {
        if block != omitted {
            *column = next_column;
            next_column += 1;
        }
    }

    // Counts into any cell are bounded by the source state's fixed outdegree.
    if max_degree <= 255 {
        match columns {
            1 => certify_counts::<u8, 1>(graph, blocks, &column_ids),
            2 => certify_counts::<u8, 2>(graph, blocks, &column_ids),
            _ => certify_counts::<u8, 3>(graph, blocks, &column_ids),
        }
    } else {
        match columns {
            1 => certify_counts::<u32, 1>(graph, blocks, &column_ids),
            2 => certify_counts::<u32, 2>(graph, blocks, &column_ids),
            _ => certify_counts::<u32, 3>(graph, blocks, &column_ids),
        }
    }
}

fn certify_counts<Count, const COLUMNS: usize>(
    graph: &impl CountRefinementGraph,
    blocks: &[u32],
    column_ids: &[usize; 4],
) -> bool
where
    Count: Copy + Default + Eq + From<u8> + std::ops::AddAssign,
{
    let mut counts = vec![Count::default(); blocks.len() * COLUMNS];
    for (target, &block) in blocks.iter().enumerate() {
        let column = column_ids[block as usize];
        if column != usize::MAX {
            graph.visit_predecessors(target as u32, |source| {
                counts[source as usize * COLUMNS + column] += Count::from(1);
            });
        }
    }

    // Fixed outdegree determines the omitted count from the other cell counts.
    let mut expected = [[Count::default(); COLUMNS]; 4];
    let mut initialized = [false; 4];
    for (&block, signature) in blocks.iter().zip(counts.chunks_exact(COLUMNS)) {
        let block = block as usize;
        if initialized[block] {
            for column in 0..COLUMNS {
                if expected[block][column] != signature[column] {
                    return false;
                }
            }
        } else {
            expected[block].copy_from_slice(signature);
            initialized[block] = true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{small_initial_stable, small_initial_stable_by_volume};
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

        fn initial_partition(&self) -> (Vec<u32>, Vec<u32>) {
            let mut ids = HashMap::new();
            let mut sizes = Vec::new();
            let blocks = (0..self.tags.len())
                .map(|state| {
                    let signature = self.initial_signature(state as u32);
                    let next_id = sizes.len() as u32;
                    let block = *ids.entry(signature).or_insert_with(|| {
                        sizes.push(0);
                        next_id
                    });
                    sizes[block as usize] += 1;
                    block
                })
                .collect();
            (blocks, sizes)
        }

        fn max_degree(&self) -> u32 {
            self.successors.iter().map(Vec::len).max().unwrap_or(0) as u32
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

    fn exact_stable(graph: &Graph, blocks: &[u32], classes: usize) -> bool {
        let mut expected = vec![None; classes];
        for (state, &block) in blocks.iter().enumerate() {
            let mut signature = vec![0_u32; classes];
            for &target in &graph.successors[state] {
                signature[blocks[target as usize] as usize] += 1;
            }
            let entry = &mut expected[block as usize];
            if let Some(expected) = entry {
                if *expected != signature {
                    return false;
                }
            } else {
                *entry = Some(signature);
            }
        }
        true
    }

    #[test]
    fn certifies_directed_tags_loops_and_repeated_edges() {
        let graph = Graph::new(
            vec![0, 0, 7, 7],
            vec![vec![0, 2, 2], vec![1, 3, 3], vec![0], vec![1]],
        );
        let (blocks, sizes) = graph.initial_partition();
        assert_eq!(sizes, [2, 2]);
        assert!(small_initial_stable(
            &graph,
            &blocks,
            &sizes,
            graph.max_degree()
        ));
    }

    #[test]
    fn rejects_equal_degrees_with_different_cell_counts() {
        let graph = Graph::new(
            vec![0, 0, 7, 7],
            vec![vec![0, 2], vec![2, 3], vec![0], vec![1]],
        );
        let (blocks, sizes) = graph.initial_partition();
        assert_eq!(sizes, [2, 2]);
        assert!(!small_initial_stable(
            &graph,
            &blocks,
            &sizes,
            graph.max_degree()
        ));
    }

    #[test]
    fn certifies_empty_regular_and_four_class_partitions() {
        for graph in [
            Graph::new(vec![], vec![]),
            Graph::new(vec![2, 2, 2], vec![vec![0, 1], vec![2, 2], vec![0, 2]]),
            Graph::new(
                vec![0, 0, 1, 1, 2, 2, 3, 3],
                vec![
                    vec![2],
                    vec![3],
                    vec![4],
                    vec![5],
                    vec![6],
                    vec![7],
                    vec![0],
                    vec![1],
                ],
            ),
        ] {
            let (blocks, sizes) = graph.initial_partition();
            assert!(small_initial_stable(
                &graph,
                &blocks,
                &sizes,
                graph.max_degree()
            ));
        }
    }

    #[test]
    fn counts_at_byte_boundary_preserve_directed_parallel_edges() {
        for degree in [255, 256] {
            let tags = vec![0, 0, 0, 0, 1, 1];
            let mut successors = vec![vec![4; degree]; 4];
            successors.extend([vec![0], vec![1]]);
            let graph = Graph::new(tags.clone(), successors.clone());
            let (blocks, sizes) = graph.initial_partition();
            assert!(small_initial_stable(&graph, &blocks, &sizes, degree as u32));
            assert!(small_initial_stable_by_volume(
                &graph,
                &blocks,
                &sizes,
                degree as u32
            ));

            successors[1] = vec![1; degree];
            let graph = Graph::new(tags, successors);
            assert!(!small_initial_stable(
                &graph,
                &blocks,
                &sizes,
                degree as u32
            ));
            assert!(!small_initial_stable_by_volume(
                &graph,
                &blocks,
                &sizes,
                degree as u32
            ));
        }
    }

    #[test]
    fn directed_random_certificates_match_full_cell_counts() {
        let mut random = 0xe210_373f_140a_bd90_u64;
        let mut next = || {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random
        };
        for _ in 0..2_000 {
            let n = (next() % 32) as usize;
            let classes = (next() % 4 + 1) as usize;
            let degrees = (0..classes)
                .map(|_| (next() % 8) as usize)
                .collect::<Vec<_>>();
            let tags = (0..n)
                .map(|_| (next() % classes as u64) as u32)
                .collect::<Vec<_>>();
            let successors = tags
                .iter()
                .map(|&tag| {
                    (0..degrees[tag as usize])
                        .map(|_| (next() % n as u64) as u32)
                        .collect()
                })
                .collect();
            let graph = Graph::new(tags, successors);
            let (blocks, sizes) = graph.initial_partition();
            assert_eq!(
                small_initial_stable(&graph, &blocks, &sizes, graph.max_degree()),
                exact_stable(&graph, &blocks, sizes.len())
            );
            assert_eq!(
                small_initial_stable_by_volume(&graph, &blocks, &sizes, graph.max_degree()),
                exact_stable(&graph, &blocks, sizes.len())
            );
        }
    }
}
