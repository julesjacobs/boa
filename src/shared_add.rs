use fxhash::FxHashMap;

use crate::binrep::{decode_header, get_header, get_state, is_state, CReader, ADD_TYP};
use crate::count_refinement::{self, CountRefinementGraph};

pub(crate) struct SharedAdd {
    tags: Vec<u32>,
    offsets: Vec<u32>,
    edges: Vec<u32>,
    reverse_offsets: Vec<u32>,
    reverse_edges: Vec<u32>,
    roots: Vec<u32>,
    reference_occurrences: usize,
}

struct Decoder<'a> {
    data: &'a [u8],
    reader: &'a CReader,
    position: usize,
    tags: Vec<u32>,
    offsets: Vec<u32>,
    edges: Vec<u32>,
    roots: Vec<u32>,
    interned: FxHashMap<&'a [u8], u32>,
    scratch: Vec<Vec<u32>>,
    reference_occurrences: usize,
}

impl<'a> Decoder<'a> {
    fn read_node(&mut self) -> Option<u32> {
        let first = *self.data.get(self.position)?;
        if first & 1 == 0 {
            self.position += 1;
            Some(self.reader.headers[(first >> 1) as usize])
        } else {
            let bytes = self
                .data
                .get(self.position..self.position.checked_add(4)?)?;
            self.position += 4;
            Some(u32::from_le_bytes(bytes.try_into().ok()?) >> 1)
        }
    }

    fn read_value(&mut self) -> Option<u64> {
        let first = *self.data.get(self.position)?;
        if first & 1 == 0 {
            self.position += 1;
            Some(self.reader.values[(first >> 1) as usize])
        } else {
            let bytes = self
                .data
                .get(self.position..self.position.checked_add(8)?)?;
            self.position += 8;
            Some(u64::from_le_bytes(bytes.try_into().ok()?) >> 1)
        }
    }

    fn collection(&mut self, start: usize, node: u32, depth: usize) -> Option<u32> {
        if is_state(node) || depth > 512 {
            return None;
        }
        let (typ, tag, len) = decode_header(get_header(node));
        if typ != ADD_TYP {
            return None;
        }
        if self.scratch.len() <= depth {
            self.scratch.push(Vec::new());
        }
        let mut children = std::mem::take(&mut self.scratch[depth]);
        children.clear();
        children.reserve(len as usize);
        for _ in 0..len {
            let child_start = self.position;
            let child = self.read_node()?;
            let target = if is_state(child) {
                self.reference_occurrences += 1;
                get_state(child).checked_mul(2)?
            } else {
                let child_id = self.collection(child_start, child, depth + 1)?;
                child_id.checked_mul(2)?.checked_add(1)?
            };
            if self.read_value()? != 1 {
                return None;
            }
            children.push(target);
        }

        let bytes = &self.data[start..self.position];
        let interned = if depth == 0 {
            None
        } else {
            self.interned.get(bytes).copied()
        };
        let id = if let Some(id) = interned {
            id
        } else {
            let id = u32::try_from(self.tags.len()).ok()?;
            id.checked_mul(2)?;
            // A reference to a root and an inline ADD child are different constructors.
            self.tags
                .push(tag as u32 | if depth == 0 { 0 } else { 1 << 8 });
            self.edges.extend_from_slice(&children);
            self.offsets.push(u32::try_from(self.edges.len()).ok()?);
            if depth != 0 {
                self.interned.insert(bytes, id);
            }
            id
        };
        self.scratch[depth] = children;
        Some(id)
    }

    fn decode(mut self) -> Option<SharedAdd> {
        while self.position < self.data.len() {
            let start = self.position;
            let node = self.read_node()?;
            let root = self.collection(start, node, 0)?;
            self.roots.push(root);
        }
        if self.interned.is_empty() {
            return None;
        }
        for target in &mut self.edges {
            *target = if *target & 1 == 0 {
                *self.roots.get((*target >> 1) as usize)?
            } else {
                *target >> 1
            };
        }

        let mut reverse_offsets = vec![0_u32; self.tags.len() + 1];
        for &target in &self.edges {
            reverse_offsets[target as usize + 1] += 1;
        }
        for state in 0..self.tags.len() {
            reverse_offsets[state + 1] += reverse_offsets[state];
        }
        let mut insertion = reverse_offsets[..self.tags.len()].to_vec();
        let mut reverse_edges = vec![0; self.edges.len()];
        for source in 0..self.tags.len() {
            for &target in
                &self.edges[self.offsets[source] as usize..self.offsets[source + 1] as usize]
            {
                let slot = &mut insertion[target as usize];
                reverse_edges[*slot as usize] = source as u32;
                *slot += 1;
            }
        }
        Some(SharedAdd {
            tags: self.tags,
            offsets: self.offsets,
            edges: self.edges,
            reverse_offsets,
            reverse_edges,
            roots: self.roots,
            reference_occurrences: self.reference_occurrences,
        })
    }
}

impl SharedAdd {
    pub(crate) fn decode(data: &[u8], reader: &CReader) -> Option<Self> {
        Decoder {
            data,
            reader,
            position: 0,
            tags: Vec::new(),
            offsets: vec![0],
            edges: Vec::new(),
            roots: Vec::new(),
            interned: FxHashMap::default(),
            scratch: Vec::new(),
            reference_occurrences: 0,
        }
        .decode()
    }

    pub(crate) fn refine(&self) -> Vec<u32> {
        let partition = count_refinement::refine_hybrid(self);
        let mut mapping = vec![u32::MAX; self.tags.len()];
        let mut next = 0;
        self.roots
            .iter()
            .map(|&root| {
                let entry = &mut mapping[partition[root as usize] as usize];
                if *entry == u32::MAX {
                    *entry = next;
                    next += 1;
                }
                *entry
            })
            .collect()
    }

    pub(crate) fn num_edges(&self) -> usize {
        self.reference_occurrences
    }

    fn successors(&self, state: u32) -> &[u32] {
        &self.edges
            [self.offsets[state as usize] as usize..self.offsets[state as usize + 1] as usize]
    }
}

impl CountRefinementGraph for SharedAdd {
    fn num_states(&self) -> usize {
        self.tags.len()
    }

    fn initial_signature(&self, state: u32) -> (u32, u32) {
        (
            self.tags[state as usize],
            self.successors(state).len() as u32,
        )
    }

    fn visit_successors(&self, state: u32, mut visit: impl FnMut(u32)) {
        for &target in self.successors(state) {
            visit(target);
        }
    }

    fn visit_predecessors(&self, state: u32, mut visit: impl FnMut(u32)) {
        for &source in &self.reverse_edges[self.reverse_offsets[state as usize] as usize
            ..self.reverse_offsets[state as usize + 1] as usize]
        {
            visit(source);
        }
    }
}

/// Refine nested unit-weight ADD input, sharing byte-identical inline children.
/// Other constructors, non-unit weights and already-flat input use the existing paths.
pub(crate) fn try_refine(data: &[u8], reader: &CReader) -> Option<Vec<u32>> {
    Some(SharedAdd::decode(data, reader)?.refine())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{try_refine, SharedAdd};
    use crate::binrep::{CWriter, Node, ADD_TYP, LIST_TYP};

    #[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
    enum Signature {
        State(u32),
        Add(u8, Vec<(Signature, u64)>),
    }

    fn signature(node: &Node, ids: &[u32]) -> Signature {
        match node {
            Node::State(state) => Signature::State(ids[*state as usize]),
            Node::Mon(ADD_TYP, tag, children) => {
                let mut counts = BTreeMap::new();
                for (child, weight) in children {
                    *counts.entry(signature(child, ids)).or_insert(0_u64) += weight;
                }
                Signature::Add(*tag, counts.into_iter().collect())
            }
            _ => panic!("Expected ADD input"),
        }
    }

    fn exact_reference(roots: &[Node]) -> Vec<u32> {
        let mut ids = vec![0; roots.len()];
        loop {
            let signatures = roots
                .iter()
                .zip(&ids)
                .map(|(root, &id)| (id, signature(root, &ids)))
                .collect::<Vec<_>>();
            let next = crate::renumber(&signatures);
            if next == ids {
                return ids;
            }
            ids = next;
        }
    }

    fn add(tag: u8, children: Vec<Node>) -> Node {
        Node::Mon(
            ADD_TYP,
            tag,
            children.into_iter().map(|child| (child, 1)).collect(),
        )
    }

    fn encode(roots: &[Node]) -> (Vec<u8>, crate::binrep::CReader) {
        let mut writer = CWriter::new();
        for root in roots {
            root.write(&mut writer);
        }
        writer.finish()
    }

    fn assert_reference(roots: &[Node]) {
        let (data, reader) = encode(roots);
        assert_eq!(try_refine(&data, &reader), Some(exact_reference(roots)));
    }

    #[test]
    fn nested_directed_cycles_multiplicity_and_tags() {
        assert_reference(&[
            add(
                0,
                vec![add(3, vec![Node::State(0), Node::State(0)]), Node::State(2)],
            ),
            add(
                0,
                vec![Node::State(3), add(3, vec![Node::State(1), Node::State(1)])],
            ),
            add(1, vec![add(3, vec![Node::State(2)])]),
            add(1, vec![add(3, vec![Node::State(3)])]),
            add(
                0,
                vec![add(4, vec![Node::State(4), Node::State(4)]), Node::State(2)],
            ),
            add(0, vec![add(3, vec![Node::State(5)]), Node::State(2)]),
        ]);
    }

    #[test]
    fn distinguishes_references_from_inline_add_nodes() {
        let roots = [
            add(0, vec![Node::State(2)]),
            add(0, vec![add(0, vec![])]),
            add(0, vec![]),
        ];
        let (data, reader) = encode(&roots);
        assert_eq!(try_refine(&data, &reader), Some(vec![0, 1, 2]));
        assert_reference(&roots);
    }

    #[test]
    fn repeated_subtrees_share_nodes_but_keep_arcs() {
        let child = || add(5, vec![Node::State(2), Node::State(2)]);
        let roots = [
            add(0, vec![child(), child()]),
            add(0, vec![child()]),
            add(1, vec![Node::State(2)]),
        ];
        let (data, reader) = encode(&roots);
        let graph = SharedAdd::decode(&data, &reader).unwrap();
        assert_eq!(graph.tags.len(), 4);
        assert_eq!(graph.edges.len(), 6);
        assert_eq!(graph.num_edges(), 7);
        assert_eq!(
            graph.successors(graph.roots[0])[0],
            graph.successors(graph.roots[0])[1]
        );
        assert_reference(&roots);
    }

    #[test]
    fn differently_ordered_auxiliary_nodes_merge_at_multiple_depths() {
        assert_reference(&[
            add(0, vec![add(7, vec![Node::State(2), Node::State(3)])]),
            add(0, vec![add(7, vec![Node::State(3), Node::State(2)])]),
            add(
                1,
                vec![add(0, vec![add(7, vec![Node::State(2), Node::State(3)])])],
            ),
            add(
                1,
                vec![add(0, vec![add(7, vec![Node::State(3), Node::State(2)])])],
            ),
        ]);
    }

    #[test]
    fn shared_nodes_support_literal_and_dictionary_encoding() {
        use crate::binrep::{encode_header, put_header, put_state};
        let reader = crate::binrep::CReader {
            headers: std::array::from_fn(|_| put_header(encode_header(ADD_TYP, 0, 1))),
            values: [1; 128],
        };
        let mut data = vec![0, 0];
        data.extend(((put_state(0) << 1) | 1).to_le_bytes());
        data.extend([0, 0, 0]);
        data.extend(((put_header(encode_header(ADD_TYP, 0, 1)) << 1) | 1).to_le_bytes());
        data.extend(((put_state(1) << 1) | 1).to_le_bytes());
        data.extend(3_u64.to_le_bytes());
        data.extend(3_u64.to_le_bytes());
        assert_eq!(try_refine(&data, &reader), Some(vec![0, 0]));
    }

    #[test]
    fn unsupported_and_truncated_inputs_use_existing_path() {
        for roots in [
            vec![add(0, vec![Node::State(0)])],
            vec![Node::Coll(LIST_TYP, 0, vec![add(0, vec![])])],
            vec![add(0, vec![Node::Coll(LIST_TYP, 0, vec![])])],
            vec![Node::Mon(ADD_TYP, 0, vec![(add(0, vec![]), 2)])],
            vec![add(
                0,
                vec![Node::Mon(ADD_TYP, 0, vec![(Node::State(0), 2)])],
            )],
            vec![add(0, vec![add(0, vec![Node::State(1)])])],
        ] {
            let (data, reader) = encode(&roots);
            assert!(try_refine(&data, &reader).is_none());
        }
        let (data, reader) = encode(&[add(0, vec![add(0, vec![Node::State(0)])])]);
        for len in 0..data.len() {
            assert!(try_refine(&data[..len], &reader).is_none());
        }
    }

    #[test]
    fn nested_random_inputs_match_exact_reference() {
        fn next(seed: &mut u64, limit: usize) -> usize {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            *seed as usize % limit
        }
        fn child(seed: &mut u64, states: usize, depth: usize) -> Node {
            if depth == 0 || next(seed, 3) == 0 {
                return Node::State(next(seed, states) as u32);
            }
            let tag = next(seed, 4) as u8;
            let children = (0..next(seed, 5))
                .map(|_| child(seed, states, depth - 1))
                .collect();
            add(tag, children)
        }
        let mut seed = 0x983d_32d1_ac68_482b;
        for _ in 0..1000 {
            let states = 1 + next(&mut seed, 20);
            let mut roots = (0..states)
                .map(|_| {
                    let tag = next(&mut seed, 4) as u8;
                    let children = (0..next(&mut seed, 5))
                        .map(|_| child(&mut seed, states, 3))
                        .collect();
                    add(tag, children)
                })
                .collect::<Vec<_>>();
            if let Node::Mon(_, _, children) = &mut roots[0] {
                children.push((add(0, vec![Node::State(0)]), 1));
            }
            assert_reference(&roots);
        }
    }
}
