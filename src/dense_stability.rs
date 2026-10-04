use datasize::DataSize;

#[derive(DataSize)]
pub(crate) struct DenseRows {
    words: usize,
    rows: Vec<u64>,
}

impl DenseRows {
    pub(crate) fn from_parts(offsets: &[u32], edges: &[u32], tags: &[u8]) -> Option<Self> {
        let n = tags.len();
        if !(256..=4096).contains(&n) || edges.len() < n * 16 {
            return None;
        }
        let mut signatures = [(0_u8, 0_u32); 4];
        let mut volumes = [0_usize; 4];
        let mut classes = 0;
        let mut max_degree = 0;
        for state in 0..n {
            let degree = offsets[state + 1] - offsets[state];
            max_degree = max_degree.max(degree);
            let signature = (tags[state], degree);
            let class = if let Some(class) = signatures[..classes]
                .iter()
                .position(|&existing| existing == signature)
            {
                class
            } else {
                if classes == 4 {
                    return None;
                }
                signatures[classes] = signature;
                classes += 1;
                classes - 1
            };
            volumes[class] += degree as usize;
        }
        if classes == 1 || max_degree <= 64 {
            return None;
        }
        let words = n.div_ceil(64);
        let sparse_work = edges.len() - *volumes[..classes].iter().max().unwrap();
        if sparse_work <= n * words * (classes - 1) {
            return None;
        }
        let mut rows = vec![0_u64; n * words];
        for state in 0..n {
            let row = &mut rows[state * words..(state + 1) * words];
            let mut previous = None;
            let mut word_index = 0;
            let mut word = 0_u64;
            for &target in &edges[offsets[state] as usize..offsets[state + 1] as usize] {
                if previous.is_some_and(|previous| previous >= target) {
                    return None;
                }
                previous = Some(target);
                let next_word = target as usize / 64;
                if next_word != word_index {
                    row[word_index] = word;
                    word_index = next_word;
                    word = 0;
                }
                word |= 1_u64 << (target % 64);
            }
            row[word_index] = word;
        }
        Some(Self { words, rows })
    }

    pub(crate) fn bytes(&self) -> usize {
        self.rows.len() * std::mem::size_of::<u64>()
    }

    pub(crate) fn certify(&self, blocks: &[u32], classes: usize) -> bool {
        debug_assert_eq!(self.rows.len(), blocks.len() * self.words);
        debug_assert!(classes <= 4);
        let mut masks = vec![0_u64; classes * self.words];
        for (state, &block) in blocks.iter().enumerate() {
            masks[block as usize * self.words + state / 64] |= 1_u64 << (state % 64);
        }
        let mut expected = [[0_u32; 4]; 4];
        let mut initialized = [false; 4];
        for (state, &block) in blocks.iter().enumerate() {
            let row = &self.rows[state * self.words..(state + 1) * self.words];
            let mut counts = [0_u32; 4];
            // Fixed outdegree determines the count into the final cell.
            for (class, count) in counts[..classes - 1].iter_mut().enumerate() {
                let mask = &masks[class * self.words..(class + 1) * self.words];
                *count = row
                    .iter()
                    .zip(mask)
                    .map(|(&a, &b)| (a & b).count_ones())
                    .sum();
            }
            let block = block as usize;
            if initialized[block] {
                if counts != expected[block] {
                    return false;
                }
            } else {
                expected[block] = counts;
                initialized[block] = true;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::DenseRows;

    #[test]
    fn sparse_class_volume_avoids_dense_rows() {
        let n = 256;
        let tags = vec![0; n];
        let mut offsets = vec![0];
        let mut edges = Vec::new();
        for state in 0..n {
            if state < 4 {
                edges.extend(4..79);
            } else {
                edges.extend(0..16);
            }
            offsets.push(edges.len() as u32);
        }
        assert!(DenseRows::from_parts(&offsets, &edges, &tags).is_none());
    }

    #[test]
    fn dense_rows_require_more_sparse_arcs_than_dense_words() {
        let n = 256;
        let tags = vec![0; n];
        let blocks = (0..n).map(|state| (state >= 8) as u32).collect::<Vec<_>>();
        for degree in [128, 129] {
            let mut offsets = vec![0];
            let mut edges = Vec::new();
            for state in 0..n {
                edges.extend(0..if state < 8 { degree } else { 16 });
                offsets.push(edges.len() as u32);
            }
            let rows = DenseRows::from_parts(&offsets, &edges, &tags);
            if degree == 128 {
                assert!(rows.is_none());
            } else {
                assert!(rows.unwrap().certify(&blocks, 2));
                edges.swap(0, 1);
                assert!(DenseRows::from_parts(&offsets, &edges, &tags).is_none());
                edges[0] = edges[1];
                assert!(DenseRows::from_parts(&offsets, &edges, &tags).is_none());
            }
        }
    }

    #[test]
    fn dense_directed_partition_and_parallel_edge_fallback() {
        let n = 256;
        let mut offsets = vec![0];
        let mut edges = Vec::new();
        let tags = (0..n).map(|v| (v >= 128) as u8).collect::<Vec<_>>();
        for state in 0..n {
            edges.extend((0..n).filter(|&v| (v + state) % 2 == 0).map(|v| v as u32));
            offsets.push(edges.len() as u32);
        }
        let rows = DenseRows::from_parts(&offsets, &edges, &tags).unwrap();
        let blocks = tags.iter().map(|&v| v as u32).collect::<Vec<_>>();
        assert!(rows.certify(&blocks, 2));
        edges[0] = 1;
        let rows = DenseRows::from_parts(&offsets, &edges, &tags).unwrap();
        let mut blocks = blocks;
        blocks[0] = 1;
        assert!(!rows.certify(&blocks, 2));
        edges[0] = edges[1];
        assert!(DenseRows::from_parts(&offsets, &edges, &tags).is_none());
    }
}
