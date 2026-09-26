//======================//
// Partition refinement //
//======================//

use std::cmp::max;
use std::hash::Hasher;
use std::hash::Hash;
use datasize::DataSize;
use itertools::Itertools;
use crate::hmap::new_hasher;
use crate::util;

use crate::{binrep::{self, CReader, get_state, is_state, decode_header, get_header, LIST_TYP, ADD_TYP, SET_TYP, MAX_TYP, OR_TYP, TAG_TYP}};


#[cfg(test)]
use crate::io;

fn ptrvec_datasize(v: &Vec<*const u8>) -> usize { v.len() * 8 }

#[derive(DataSize)]
pub struct Coalg {
    pub data: Vec<u8>, // binary representation of the coalgebra
    pub reader: binrep::CReader,
    #[data_size(with = ptrvec_datasize)]
    pub locs: Vec<*const u8>, // gives the location in data where the i-th state starts
    pub backrefs: Vec<u32>, // buffer of backrefs
    pub backrefs_locs: Vec<u32> // backrefs_locs[i] gives the index into backrefs[backrefs_locs[i]] where the backrefs of the i-th state start
}


impl Coalg {
    pub fn new(data: Vec<u8>, r: CReader) -> Coalg {
        // Iterate over one node starting at p, calling f(i) on each state ref @i in the node.
        // State refs are handled inline; only nested nodes need a recursive call.
        #[inline(always)]
        unsafe fn iter<F>(p: &mut *const u8, r: &CReader, f : &mut F)
        where F : FnMut(u32) -> () {
            let w = r.read_node_mut(p);
            if is_state(w) {
                f(get_state(w));
            } else {
                iter_node(p, r, f, get_header(w));
            }
        }

        unsafe fn iter_node<F>(p: &mut *const u8, r: &CReader, f : &mut F, header: u32)
        where F : FnMut(u32) -> () {
            let (typ,_tag,len) = decode_header(header);
            match typ {
                LIST_TYP|SET_TYP => {
                    for _ in 0..len {
                        iter(p,r,f)
                    }
                },
                ADD_TYP|MAX_TYP|OR_TYP|TAG_TYP => {
                    for _ in 0..len {
                        iter(p,r,f);
                        r.read_value_mut(p);
                    }
                },
                _ => {
                    panic!("Unknown typ.")
                }
            }
        }

        let mut locs = vec![];
        let mut backrefs_locs: Vec<u32> = vec![];

        unsafe {
            // Compute number of backrefs to state i in backrefs_locs[i]
            // Also computes locs[i] pointers to beginning of state i
            let mut p = data.as_ptr();
            let mut state_num:u32 = 0;
            while !CReader::is_at_end(&data,p) {
                locs.push(p);
                iter(&mut p, &r, &mut |w| {
                    while w as usize >= backrefs_locs.len() { backrefs_locs.push(0) }
                    backrefs_locs[w as usize] += 1;
                });
                state_num += 1;
            }
            while backrefs_locs.len() <= state_num as usize { backrefs_locs.push(0) }

            // Compute cumulative sum
            let mut total_backrefs = 0;
            for i in 0..backrefs_locs.len() {
                total_backrefs += backrefs_locs[i];
                backrefs_locs[i] = total_backrefs;
            }

            let mut backrefs = vec![0;total_backrefs as usize];

            // Fill in the actual backrefs
            let mut p = data.as_ptr();
            let mut state_num:u32 = 0;
            while !CReader::is_at_end(&data,p) {
                iter(&mut p, &r, &mut |w| {
                    // state_num refers to state w
                    backrefs_locs[w as usize] -= 1;
                    backrefs[backrefs_locs[w as usize] as usize] = state_num;
                });
                state_num += 1;
            }
            debug_assert_eq!(backrefs_locs.len(), state_num as usize + 1);

            Coalg {
                data: data,
                reader: r,
                locs: locs,
                backrefs: backrefs,
                backrefs_locs: backrefs_locs
            }
        }
    }

    pub fn state_backrefs(self: &Self, state: u32) -> &[u32] {
        let start = self.backrefs_locs[state as usize];
        let end = self.backrefs_locs[state as usize + 1];
        return &self.backrefs[start as usize..end as usize];
    }

    pub fn num_states(self: &Self) -> u32 {
        return self.locs.len() as u32
    }

    pub fn dump(self: &Self) {
        println!("Coalg {{\n  data: {:?},\n  locs: {:?},\n  backrefs: {:?},\n  backrefs_locs: {:?}\n}}", self.data, self.locs, self.backrefs, self.backrefs_locs);
    }

    pub fn dump_backrefs(self: &Self) {
        for state in 0..self.num_states() {
            println!("@{} backrefs={:?}", state, self.state_backrefs(state));
        }
    }
}

#[test]
fn test_new_coalg() {
    let (data,r) = io::read_boa_txt("tests/test1.boa.txt");
    // 0: List[0]{@0,@1}
    // 1: List[0]{@1,@1}
    // 2: List[1]{@0,@0}
    // 3: List[1]{@0,@0}
    // 4: List[1]{@3,@4}
    // 5: Add[0]{@0:1,@1:1}
    // 6: Add[0]{@0:2}
    // 7: Add[0]{@0:2,@1:1}
    let coa = Coalg::new(data,r);
    assert_eq!(coa.num_states(), 8);
    assert_eq!(&coa.backrefs, &vec![7,6,5,3,3,2,2,0,  7,5,1,1,0,  4,  4]); // 0,2,2,3,3,5,6,7,  0,1,1,5,7,  4,  4
    assert_eq!(&coa.backrefs_locs, &vec![0,8,13,13,14,15,15,15,15]); // note that the states 5,6,7 have no corresponding entry because they are never referred to and are at the end
    assert_eq!(&coa.state_backrefs(0), &vec![7,6,5,3,3,2,2,0]);
}

type ID = u32; // represents canonical ID of a state or sub-node of a state, refers to a partition number

/// Source of the signature of a state reference @i inside a node.
pub trait Ids {
    fn id(&self, state: u32) -> u64;
}

impl Ids for [u32] {
    #[inline(always)]
    fn id(&self, state: u32) -> u64 { self[state as usize] as u64 }
}

impl Ids for [u64] {
    #[inline(always)]
    fn id(&self, state: u32) -> u64 { self[state as usize] }
}

/// Every state has the same signature; used to compute the initial partition.
pub struct InitIds;

impl Ids for InitIds {
    #[inline(always)]
    fn id(&self, _state: u32) -> u64 { 0 }
}

/// Hash a sorted slice of (key,value) pairs, combining the values of equal keys with op.
fn hash_with_op<A,F,H>(repr: &[(A,u64)], hasher: &mut H, op: F)
where F : Fn(u64,u64) -> u64, A:Eq+Copy+Hash, H:Hasher {
    let mut i = 0;
    while i < repr.len() {
        let (x,v) = repr[i];
        let mut vtot = v;
        i += 1;
        while i < repr.len() {
            let (x2,v2) = repr[i];
            if x == x2 {
                i += 1;
                vtot = op(vtot,v2);
            } else { break }
        }
        (x,vtot).hash(hasher);
    }
}

/// Hash each distinct element of a sorted slice.
fn hash_dedup<A:Eq+Copy+Hash, H:Hasher>(repr: &[A], hasher: &mut H) {
    let mut prev: Option<A> = None;
    for &x in repr {
        if prev != Some(x) {
            x.hash(hasher);
            prev = Some(x);
        }
    }
}

/// Computes the signature of the node header w whose children start at p.
/// Children of sets and monoids are collected on top of the scratch stack,
/// so no allocation is needed once the scratch buffer has grown large enough.
unsafe fn canonicalize_node<I:Ids+?Sized>(mut p : *const u8, r: &CReader, ids: &I, w: u32, scratch: &mut Vec<(u64,u64)>) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    if typ == LIST_TYP {
        for _ in 0..len {
            let (sig, rest) = canonicalize(p, r, ids, scratch);
            sig.hash(&mut hasher);
            p = rest;
        }
        return (hasher.finish(), p);
    }
    let base = scratch.len();
    match typ {
        SET_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize(p, r, ids, scratch);
                scratch.push((sig,0));
                p = rest;
            }
            let repr = &mut scratch[base..];
            repr.sort_unstable();
            for &(sig,_) in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP|TAG_TYP => {
            for _ in 0..len {
                let (sig, p2) = canonicalize(p, r, ids, scratch);
                let (w,p3) = r.read_value(p2);
                p = p3;
                scratch.push((sig,w));
            }
            let repr = &mut scratch[base..];
            // Sorting whole pairs keeps equal keys adjacent; the ops below are commutative.
            repr.sort_unstable();
            match typ {
                ADD_TYP => hash_with_op(repr, &mut hasher, |a,b| a.wrapping_add(b)),
                OR_TYP => hash_with_op(repr, &mut hasher, |a,b| a|b),
                MAX_TYP => hash_with_op(repr, &mut hasher, |a,b| max(a,b)),
                _ => hash_dedup(repr, &mut hasher),
            }
        },
        _ => panic!("Unknown typ.")
    }
    scratch.truncate(base);
    return (hasher.finish(), p);
}

/// Computes the signature of the node starting at p, and returns a pointer just past it.
#[inline(always)]
unsafe fn canonicalize<I:Ids+?Sized>(p : *const u8, r: &CReader, ids: &I, scratch: &mut Vec<(u64,u64)>) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (ids.id(get_state(w)), p);
    } else {
        return canonicalize_node(p, r, ids, get_header(w), scratch);
    }
}

/// Computes the signatures of the given states into sigs (which is cleared first).
pub fn repartition_into(coa : &Coalg, states: &[u32], ids: &[ID], sigs: &mut Vec<u64>, scratch: &mut Vec<(u64,u64)>) {
    sigs.clear();
    sigs.reserve(states.len());
    // The states are scattered in memory, so prefetch the data of states a few iterations ahead,
    // and the location of that data a few iterations before that.
    const AHEAD: usize = 4;
    for &state in states.iter().take(2 * AHEAD) {
        util::prefetch(&coa.locs[state as usize]);
    }
    for &state in states.iter().take(AHEAD) {
        util::prefetch(coa.locs[state as usize]);
    }
    for (i, &state) in states.iter().enumerate() {
        if let Some(&next) = states.get(i + 2 * AHEAD) {
            util::prefetch(&coa.locs[next as usize]);
        }
        if let Some(&next) = states.get(i + AHEAD) {
            util::prefetch(coa.locs[next as usize]);
        }
        let p = coa.locs[state as usize];
        unsafe {
            let (sig,_rest) = canonicalize(p, &coa.reader, ids, scratch);
            sigs.push(sig);
        }
    }
}

pub fn repartition_unsafe(coa : &Coalg, states: &[u32], ids: &[ID]) -> Vec<u64> {
    let mut sigs = vec![];
    repartition_into(coa, states, ids, &mut sigs, &mut vec![]);
    return sigs
}

fn repartition_all<I:Ids+?Sized>(data: &[u8], r: &CReader, ids: &I) -> Vec<u64> {
    let mut scratch = vec![];
    let mut new_ids_raw = vec![];
    unsafe {
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let (sig, p_next) = canonicalize(p, r, ids, &mut scratch);
            new_ids_raw.push(sig);
            p = p_next;
        }
    }
    return new_ids_raw
}

pub fn repartition_all_unsafe64(data: &[u8], r: &CReader, ids: &[u64]) -> Vec<u64> {
    repartition_all(data, r, ids)
}

pub fn init_partition_ids_unsafe(data: &[u8], r: &CReader) -> Vec<u64> {
    repartition_all(data, r, &InitIds)
}
