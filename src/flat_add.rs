use crate::binrep::{CReader, ADD_TYP};
use crate::count_refinement::CountRefinementGraph;

pub(crate) struct FlatUnitAdd {
    dense_rows: Option<crate::dense_stability::DenseRows>,
    offsets: Vec<u32>,
    edges: Vec<u32>,
    tags: Vec<u8>,
    reverse: Option<(Vec<u32>, Vec<u32>)>,
}

struct Decoder<'a> {
    data: &'a [u8],
    reader: &'a CReader,
    position: usize,
}

impl Decoder<'_> {
    fn node(&mut self) -> Option<u32> {
        let first = *self.data.get(self.position)?;
        if first & 1 == 0 {
            self.position += 1;
            Some(self.reader.headers[(first >> 1) as usize])
        } else {
            let end = self.position.checked_add(4)?;
            let raw = self.data.get(self.position..end)?;
            self.position = end;
            Some(u32::from_le_bytes(raw.try_into().ok()?) >> 1)
        }
    }

    fn value(&mut self) -> Option<u64> {
        let first = *self.data.get(self.position)?;
        if first & 1 == 0 {
            self.position += 1;
            Some(self.reader.values[(first >> 1) as usize])
        } else {
            let end = self.position.checked_add(8)?;
            let raw = self.data.get(self.position..end)?;
            self.position = end;
            Some(u64::from_le_bytes(raw.try_into().ok()?) >> 1)
        }
    }
}

impl FlatUnitAdd {
    pub(crate) fn from_coalg(data: &[u8], reader: &CReader, symmetric: bool) -> Option<Self> {
        let mut decoder = Decoder {
            data,
            reader,
            position: 0,
        };
        if !data.is_empty() {
            let node = decoder.node()?;
            if node & 1 == 0 || ((node >> 1) >> 24) as u8 != ADD_TYP {
                return None;
            }
        }
        decoder.position = 0;
        let mut offsets = Vec::with_capacity(data.len() / 20 + 1);
        let mut edges = Vec::<u32>::with_capacity(data.len() / 5);
        let mut tags = Vec::with_capacity(data.len() / 20);
        let unit_weight = reader
            .values
            .iter()
            .position(|&weight| weight == 1)
            .map(|i| (2 * i) as u8);
        let mut max_target = 0;
        offsets.push(0);
        while decoder.position < data.len() {
            let node = decoder.node()?;
            if node & 1 == 0 {
                return None;
            }
            let header = node >> 1;
            if (header >> 24) as u8 != ADD_TYP || tags.len() >= u32::MAX as usize {
                return None;
            }
            let len = header as u16 as usize;
            let end = edges
                .len()
                .checked_add(len)
                .filter(|&end| end <= u32::MAX as usize)?;
            edges.reserve(len);
            let mut fast = false;
            if let Some(unit_weight) = unit_weight {
                let row_end = decoder.position.checked_add(len * 5)?;
                if let Some(row) = data.get(decoder.position..row_end) {
                    let mut invalid = 0;
                    let mut row_max = 0;
                    // The complete row bounds each read, and reserve bounds each write.
                    unsafe {
                        let output = edges.as_mut_ptr().add(edges.len());
                        let input = row.as_ptr();
                        for index in 0..len {
                            let reference =
                                u32::from_le(input.add(index * 5).cast::<u32>().read_unaligned());
                            invalid |= (reference & 3) ^ 1;
                            invalid |= (*input.add(index * 5 + 4) ^ unit_weight) as u32;
                            let target = reference >> 2;
                            output.add(index).write(target);
                            row_max = row_max.max(target);
                        }
                        if invalid == 0 {
                            edges.set_len(end);
                            max_target = max_target.max(row_max);
                            decoder.position = row_end;
                            fast = true;
                        }
                    }
                }
            }
            if !fast {
                for _ in 0..len {
                    let child = decoder.node()?;
                    if child & 1 != 0 || decoder.value()? != 1 {
                        return None;
                    }
                    let target = child >> 1;
                    edges.push(target);
                    max_target = max_target.max(target);
                }
            }
            tags.push((header >> 16) as u8);
            offsets.push(end as u32);
        }
        if !edges.is_empty() && max_target as usize >= tags.len() {
            return None;
        }
        let reverse = if symmetric {
            None
        } else {
            let mut counts = vec![0_u32; tags.len()];
            for &target in &edges {
                counts[target as usize] += 1;
            }
            let mut reverse_offsets = Vec::with_capacity(tags.len() + 1);
            reverse_offsets.push(0_u32);
            for count in &mut counts {
                let start = *reverse_offsets.last().unwrap();
                reverse_offsets.push(start + *count);
                *count = start;
            }
            let mut reverse_edges = vec![0; edges.len()];
            for source in 0..tags.len() {
                for &target in &edges[offsets[source] as usize..offsets[source + 1] as usize] {
                    let slot = &mut counts[target as usize];
                    reverse_edges[*slot as usize] = source as u32;
                    *slot += 1;
                }
            }
            Some((reverse_offsets, reverse_edges))
        };
        Some(Self {
            dense_rows: crate::dense_stability::DenseRows::from_parts(&offsets, &edges, &tags),
            offsets,
            edges,
            tags,
            reverse,
        })
    }

    fn graph_view(&self) -> GraphView<'_> {
        let (predecessor_offsets, predecessor_edges) = self
            .reverse
            .as_ref()
            .map_or((&self.offsets[..], &self.edges[..]), |(offsets, edges)| {
                (&offsets[..], &edges[..])
            });
        GraphView {
            dense_rows: self.dense_rows.as_ref(),
            tags: &self.tags,
            offsets: &self.offsets,
            edges: &self.edges,
            predecessor_offsets,
            predecessor_edges,
        }
    }

    pub(crate) fn refine_counts(&self) -> Vec<u32> {
        crate::count_refinement::refine_hybrid(&self.graph_view())
    }

    pub(crate) fn num_states(&self) -> usize {
        self.tags.len()
    }

    pub(crate) fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub(crate) fn representation_bytes(&self) -> usize {
        let reverse_bytes = self.reverse.as_ref().map_or(0, |(offsets, edges)| {
            offsets.len() * std::mem::size_of::<u32>() + edges.len() * std::mem::size_of::<u32>()
        });
        std::mem::size_of::<Self>()
            + self.offsets.len() * std::mem::size_of::<u32>()
            + self.edges.len() * std::mem::size_of::<u32>()
            + self.tags.len()
            + reverse_bytes
            + self.dense_rows.as_ref().map_or(0, |rows| rows.bytes())
    }
}

struct GraphView<'a> {
    dense_rows: Option<&'a crate::dense_stability::DenseRows>,
    tags: &'a [u8],
    offsets: &'a [u32],
    edges: &'a [u32],
    predecessor_offsets: &'a [u32],
    predecessor_edges: &'a [u32],
}

impl CountRefinementGraph for GraphView<'_> {
    fn dense_rows(&self) -> Option<&crate::dense_stability::DenseRows> {
        self.dense_rows
    }
    fn num_states(&self) -> usize {
        self.tags.len()
    }

    fn initial_signature(&self, state: u32) -> (u32, u32) {
        let state = state as usize;
        (
            self.tags[state] as u32,
            self.offsets[state + 1] - self.offsets[state],
        )
    }

    fn visit_predecessors(&self, state: u32, mut visit: impl FnMut(u32)) {
        let state = state as usize;
        for &source in &self.predecessor_edges
            [self.predecessor_offsets[state] as usize..self.predecessor_offsets[state + 1] as usize]
        {
            visit(source);
        }
    }

    fn visit_successors(&self, state: u32, mut visit: impl FnMut(u32)) {
        let state = state as usize;
        for &target in &self.edges[self.offsets[state] as usize..self.offsets[state + 1] as usize] {
            visit(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FlatUnitAdd;
    use crate::binrep::{CReader, CWriter, Node, ADD_TYP, LIST_TYP};
    use crate::count_refinement::CountRefinementGraph;

    fn encode(tags: &[u8], adjacency: &[Vec<u32>]) -> (Vec<u8>, CReader) {
        let mut writer = CWriter::new();
        for (state, targets) in adjacency.iter().enumerate() {
            Node::Mon(
                ADD_TYP,
                tags[state],
                targets
                    .iter()
                    .map(|&target| (Node::State(target), 1))
                    .collect(),
            )
            .write(&mut writer);
        }
        writer.finish()
    }

    #[test]
    fn directed_random_partitions_match_exact_reference() {
        let mut random = 0x195f_3ade_2100_a11d_u64;
        let mut next = || {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random
        };
        for _ in 0..1_000 {
            let n = (next() % 32) as usize;
            let tags = (0..n).map(|_| (next() % 4) as u8).collect::<Vec<_>>();
            let adjacency = (0..n)
                .map(|_| {
                    (0..next() % 20)
                        .map(|_| (next() % n as u64) as u32)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let (data, reader) = encode(&tags, &adjacency);
            let graph = FlatUnitAdd::from_coalg(&data, &reader, false).unwrap();
            let mut blocks = vec![0; n];
            loop {
                let mut signatures = std::collections::HashMap::new();
                let next_blocks = (0..n)
                    .map(|state| {
                        let mut targets = adjacency[state]
                            .iter()
                            .map(|&target| blocks[target as usize])
                            .collect::<Vec<_>>();
                        targets.sort_unstable();
                        let next_id = signatures.len() as u32;
                        *signatures.entry((tags[state], targets)).or_insert(next_id)
                    })
                    .collect::<Vec<_>>();
                if blocks == next_blocks {
                    break;
                }
                blocks = next_blocks;
            }
            assert_eq!(graph.refine_counts(), blocks);
        }
    }

    #[test]
    fn builds_directed_reverse_edges_with_multiplicity() {
        let adjacency = vec![vec![2, 2, 0], vec![2], vec![], vec![1, 0]];
        let (data, reader) = encode(&[0, 7, 0, 7], &adjacency);
        let graph = FlatUnitAdd::from_coalg(&data, &reader, false).unwrap();
        assert_eq!(graph.tags, [0, 7, 0, 7]);
        for state in 0..4 {
            let mut actual = Vec::new();
            graph
                .graph_view()
                .visit_predecessors(state, |source| actual.push(source));
            let expected = adjacency
                .iter()
                .enumerate()
                .flat_map(|(source, edges)| {
                    edges
                        .iter()
                        .filter(move |&&target| target == state)
                        .map(move |_| source as u32)
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn preserves_other_functors_and_weights_for_generic_fallback() {
        for node in [
            Node::Coll(LIST_TYP, 0, vec![]),
            Node::Mon(ADD_TYP, 0, vec![(Node::State(0), 2)]),
            Node::Mon(ADD_TYP, 0, vec![(Node::Mon(ADD_TYP, 0, vec![]), 1)]),
        ] {
            let mut writer = CWriter::new();
            node.write(&mut writer);
            let (data, reader) = writer.finish();
            assert!(FlatUnitAdd::from_coalg(&data, &reader, false).is_none());
        }
    }

    #[test]
    fn rejects_truncation_and_invalid_references() {
        let (data, reader) = encode(&[0], &[vec![0, 0]]);
        for end in 1..data.len() {
            assert!(FlatUnitAdd::from_coalg(&data[..end], &reader, false).is_none());
        }
        let (data, reader) = encode(&[0], &[vec![1]]);
        assert!(FlatUnitAdd::from_coalg(&data, &reader, false).is_none());
    }
}
