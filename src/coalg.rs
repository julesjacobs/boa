//======================//
// Partition refinement //
//======================//

use std::cmp::max;
use std::hash::Hasher;
use std::hash::Hash;
use datasize::DataSize;
use itertools::Itertools;
use crate::hmap::new_hasher;

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
        // Iterate over one state starting at data[loc], calling f(i) on each state ref @i in the state.
        unsafe fn iter<F>(p: &mut *const u8, r: &CReader, f : &mut F)
        where F : FnMut(u32) -> () {
            let w = r.read_node_mut( p);
            if is_state(w) {
                f(get_state(w));
            } else {
                let (typ,_tag,len) = decode_header(get_header(w));
                match typ {
                    LIST_TYP|SET_TYP => {
                        for _ in 0..len {
                            iter(p,r,f)
                        }
                    },
                    ADD_TYP|MAX_TYP|OR_TYP|TAG_TYP => {
                        for _ in 0..len {
                            iter(p,r,f);
                            r.read_value_mut( p);
                        }
                    },
                    _ => {
                        panic!("Unknown typ.")
                    }
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

fn hash_with_op<A,F,H>(repr: &mut [(A,u64)], hasher: &mut H, op: F)
where F : Fn(u64,u64) -> u64, A:Ord+Copy+Hash, H:Hasher {
    repr.sort_by_key(|kv| kv.0);
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

unsafe fn canonicalize_node_unsafe<'a>(mut p : *const u8, r: &CReader, ids: &[ID], w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_unsafe(p, r, ids);
                sig.hash(&mut hasher);
                p = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_unsafe(p, r, ids);
                p = rest; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe(p, r, ids);
                let (w,p3) = r.read_value(p2);
                p = p3;
                (sig,w)
            }).collect();
            match typ {
                ADD_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a+b),
                OR_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a|b),
                MAX_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b)),
                _ => panic!("Unreachable")
            }
        },
        TAG_TYP => {
            let mut repr : Vec<u64> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe(p, r, ids);
                let (w,p3) = r.read_value(p2);
                p = p3;
                // (sig,w)
                let mut h = new_hasher();
                sig.hash(&mut h);
                w.hash(&mut h);
                h.finish()
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

unsafe fn canonicalize_unsafe<'a>(p : *const u8, r: &CReader, ids: &[ID]) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (ids[get_state(w) as usize] as u64, p);
    } else {
        return canonicalize_node_unsafe(p, r, ids, get_header(w));
    }
}

pub fn repartition_unsafe(coa : &Coalg, states: &[u32], ids: &[ID]) -> Vec<u64> {
    let mut sigs = vec![];
    sigs.reserve(states.len());
    for &state in states {
        let p = coa.locs[state as usize];
        unsafe {
            let (sig,_rest) = canonicalize_unsafe(p, &coa.reader, ids);
            sigs.push(sig);
        }
    }
    return sigs
}

unsafe fn canonicalize_node_unsafe64<'a>(mut p : *const u8, r: &CReader, ids: &[u64], w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_unsafe64(p, r, ids);
                sig.hash(&mut hasher);
                p = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_unsafe64(p, r, ids);
                p = rest; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe64(p, r, ids);
                let (w,p3) = r.read_value(p2);
                p = p3;
                (sig,w)
            }).collect();
            match typ {
                ADD_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a+b),
                OR_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a|b),
                MAX_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b)),
                _ => panic!("Unreachable")
            }
        },
        TAG_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe64(p, r, ids);

                // let x = *(p as *const u32);
                // let w = get_noncompressed32(x);
                // let p2 = p.add(4);
                // // let (w,p2) = r.read_node(p);
                // let sig = ids[get_state(w) as usize] as u64;

                // unsafe fn read_node(self: &Self, data: *const u8) -> (u32, *const u8) {
                //     let x = *(data as *const u32);
                //     if is_compressed32(x) { (self.headers[get_compressed32(x) as usize], data.add(1)) }
                //     else { (get_noncompressed32(x), data.add(4)) }
                // }
                // unsafe fn canonicalize_unsafe64<'a>(p : *const u8, r: &CReader, ids: &[u64]) -> (u64, *const u8) {
                //     let (w,p) = r.read_node(p);
                //     if is_state(w) {
                //         return (ids[get_state(w) as usize] as u64, p);
                //     } else {
                //         return canonicalize_node_unsafe64(p, r, ids, get_header(w));
                //     }
                // }

                let (w,p3) = r.read_value(p2);
                p = p3;
                (sig,w)
                // let mut h = new_hasher();
                // sig.hash(&mut h);
                // w.hash(&mut h);
                // h.finish()
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

pub unsafe fn canonicalize_unsafe64<'a>(p : *const u8, r: &CReader, ids: &[u64]) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (ids[get_state(w) as usize] as u64, p);
    } else {
        return canonicalize_node_unsafe64(p, r, ids, get_header(w));
    }
}

pub fn repartition_unsafe64(coa : &Coalg, states: &[u32], ids: &[u64]) -> Vec<u64> {
    let mut sigs = vec![];
    sigs.reserve(states.len());
    for &state in states {
        let p = coa.locs[state as usize];
        unsafe {
            let (sig,_rest) = canonicalize_unsafe64(p, &coa.reader, ids);
            sigs.push(sig);
        }
    }
    return sigs;
}

pub fn repartition_all_unsafe64(data: &[u8], r: &CReader, ids: &[u64]) -> Vec<u64> {
    unsafe {
        let mut new_ids_raw = vec![];
        new_ids_raw.reserve(ids.len());
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let (sig, p_next) = canonicalize_unsafe64(p, r, ids);
            new_ids_raw.push(sig);
            p = p_next;
        }
        return new_ids_raw
    }
}

pub unsafe fn canonicalize_node_unsafe_init<'a>(mut p : *const u8, r: &CReader, w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, p2) = canonicalize_unsafe_init(p, r);
                sig.hash(&mut hasher);
                p = p2;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe_init(p, r);
                p = p2; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe_init(p, r);
                let (w,p3) = r.read_value(p2);
                p = p3;
                (sig,w)
            }).collect();
            match typ {
                ADD_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a+b),
                OR_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| a|b),
                MAX_TYP => hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b)),
                _ => panic!("Unreachable")
            }
        },
        TAG_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_unsafe_init(p, r);
                let (w,p3) = r.read_value(p2);
                p = p3;
                (sig,w)
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

pub unsafe fn canonicalize_unsafe_init<'a>(p : *const u8, r: &CReader) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (0, p);
    } else {
        return canonicalize_node_unsafe_init(p, r, get_header(w));
    }
}

pub fn init_partition_ids_unsafe(data: &[u8], r: &CReader) -> Vec<u64> {
    unsafe {
        let mut new_ids_raw = vec![];
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let (sig, p_next) = canonicalize_unsafe_init(p, r);
            new_ids_raw.push(sig);
            p = p_next;
        }
        return new_ids_raw
    }
}