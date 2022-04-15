#![allow(dead_code)]
use std::{hash::{Hash, Hasher}, fs::File, io::{BufReader, BufRead}, path::Path, cmp::max, time::SystemTime, env};
use ahash::AHasher;
use ahash::AHashMap;

// This hasher is much faster than the default one
fn new_hasher() -> AHasher { AHasher::new_with_keys(1234, 5678) }
type HMap<K,V> = AHashMap<K,V>;

// Using a different allocator also makes a huge difference
// I've found jemalloc to be better than mimalloc, both in terms of speed and memory use
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// Binary representation & Parsing
// ===============================

/// Checks if the word w is a state or not
fn is_state(w: u32) -> bool {
    return w >> 31 == 0;
}

/// Computes a state's word
fn state_w(w: u32) -> u32 {
    debug_assert!(w >> 31 == 0);
    return w
}

/// Computes a non-state's word
fn nonstate_w(w: u32) -> u32 {
    debug_assert!(w >> 31 == 0);
    return w | (1 << 31);
}

/// Computes a collection header word
/// * typ has to be 0..128
fn coll_w(typ: u8, tag: u8, len: u16) -> u32 {
    debug_assert!(typ <= 128);
    return nonstate_w((u32::from(typ)<<24) + (u32::from(tag)<<16) + u32::from(len));
}

fn get_typ(w: u32) -> u8 {
    return (w>>24 & 0b01111111) as u8
}

fn get_tag(w: u32) -> u8 {
    return (w>>16) as u8
}

fn get_len(w: u32) -> u16 {
    return w as u16
}

const LIST_TYP: u8 = 0;
const SET_TYP: u8 = 1;
const ADD_TYP: u8 = 2;
const MAX_TYP: u8 = 3;
const OR_TYP: u8 = 4;

#[test]
fn test_binrep(){
    assert_eq!(coll_w(2, 3, 8), 0x82030008);
    assert_eq!(state_w(34234234), 34234234);
    assert_eq!(get_typ(coll_w(2, 3, 8)), 2);
    assert_eq!(get_tag(coll_w(2, 3, 8)), 3);
    assert_eq!(get_len(coll_w(2, 3, 8)), 8);
    assert_eq!(get_typ(coll_w(LIST_TYP, 3, 8)), LIST_TYP);
}



#[derive(Debug)]
enum Node {
    State(u32),
    Coll(u8, u8, Vec<Node>),
    Mon(u8, u8, Vec<(Node,u64)>)
}

fn node_to_string(n: Node, buf: &mut String) {
    match n {
        Node::State(s) => {
            buf.push_str("@");
            buf.push_str(&s.to_string())
        }
        Node::Coll(typ,tag,ns) => {
            match typ {
                LIST_TYP => buf.push_str("List"),
                SET_TYP => buf.push_str("Set"),
                _ => panic!("Bad typ for Coll")
            }
            buf.push_str("["); buf.push_str(&tag.to_string()); buf.push_str("]");
            buf.push_str("{");
            let mut is_first = true;
            for n2 in ns {
                if is_first { is_first = false; }
                else { buf.push_str(","); }
                node_to_string(n2,buf);
            }
            buf.push_str("}");
        }
        Node::Mon(typ,tag,cs) => {
            match typ {
                ADD_TYP => buf.push_str("Add"),
                MAX_TYP => buf.push_str("Max"),
                OR_TYP => buf.push_str("Or"),
                _ => panic!("Bad typ for Mon")
            }
            buf.push_str("["); buf.push_str(&tag.to_string()); buf.push_str("]");
            buf.push_str("{");
            let mut is_first = true;
            for (n2,x2) in cs {
                if is_first { is_first = false; }
                else { buf.push_str(","); }
                node_to_string(n2,buf);
                buf.push_str(":");
                buf.push_str(&x2.to_string());
            }
            buf.push_str("}");
        }
    }
}

fn node_to_bin(n: &Node, buf: &mut Vec<u32>) {
    match n {
        Node::State(s) => {
            buf.push(state_w(*s))
        }
        Node::Coll(typ,tag,ns) => {
            buf.push(coll_w(*typ,*tag,ns.len() as u16));
            for n2 in ns {
                node_to_bin(&n2,buf)
            }
        }
        Node::Mon(typ,tag,cs) => {
            buf.push(coll_w(*typ,*tag,cs.len() as u16));
            for (n2,x2) in cs {
                node_to_bin(n2,buf);
                buf.push(*x2 as u32);
                buf.push((*x2 >> 32) as  u32);
            }
        }
    }
}

fn peek_c(buf: &[char], i: &mut usize) -> Option<char> {
    if *i >= buf.len() {
        return None
    }else{
        let w = buf[*i];
        return Some(w);
    }
}
fn get_c(buf: &[char], i: &mut usize) -> Option<char> {
    if *i >= buf.len() {
        return None
    }else{
        let w = buf[*i];
        *i += 1;
        return Some(w);
    }
}
fn parse_error<T>(buf: &[char], i: &mut usize, msg: &str) -> T {
    let mut ind = String::from("");
    for _ in 0..*i {
        ind.push_str(" ");
    }
    ind.push_str("^");
    panic!("Parse error: {} \nError occurred at position {} in:\n{}\n{}", msg, i, String::from_iter(buf), ind)
}
fn expect_str(buf: &[char], i: &mut usize, s: &str) {
    for c in s.chars() {
        match get_c(buf,i) {
            None => parse_error(buf,i,&format!("Unexpected end of input while expecting {}.", s)),
            Some(c2) => if c == c2 {} else { *i -= 1; parse_error(buf,i,&format!("Unexpected character while expecting {}.", s)) }
        }
    }
}

fn parse_num(buf: &[char], i: &mut usize) -> u64 {
    let mut num : u64 = 0;
    loop {
        match peek_c(buf,i) {
            None => break,
            Some(c) => {
                match char::to_digit(c, 10) {
                    Some(k) => { *i += 1; num = num*10 + (k as u64); }
                    None => break
                }
            }
        }
    }
    return num
}

fn parse_tag(buf: &[char], i: &mut usize) -> u8 {
    expect_str(buf,i,"[");
    let n = parse_num(buf,i);
    if n > 255 {
        return parse_error(buf,i,"Tag values must be in the range 0 - 255.");
    }
    expect_str(buf,i,"]");
    return n as u8
}

fn parse_coll(buf: &[char], i: &mut usize) -> Vec<Node> {
    expect_str(buf,i,"{");
    let mut ns = vec![];
    match peek_c(buf,i) {
        Some('}') => { *i += 1; return ns }
        _ => {}
    }
    loop {
        ns.push(parse_node(buf,i));
        match get_c(buf,i) {
            Some('}') => return ns,
            Some(',') => {},
            Some(_) => parse_error(buf,i,"Unexpected character in collection."),
            None => parse_error(buf,i,"Unexpected end of input."),
        }
    }
}

fn parse_mon(buf: &[char], i: &mut usize) -> Vec<(Node,u64)> {
    expect_str(buf,i,"{");
    let mut ns = vec![];
    match peek_c(buf,i) {
        Some('}') => { *i += 1; return ns }
        _ => {}
    }
    loop {
        let n = parse_node(buf,i);
        expect_str(buf,i,":");
        let x = parse_num(buf,i);
        ns.push((n,x));
        match get_c(buf,i) {
            Some('}') => return ns,
            Some(',') => {},
            Some(_) => parse_error(buf,i,"Unexpected character in monoid collection."),
            None => parse_error(buf,i,"Unexpected end of input."),
        }
    }
}

fn parse_node(buf: &[char], i: &mut usize) -> Node {
    match get_c(buf,i) {
        Some('@') => {
            let n = parse_num(buf,i);
            if n > 2147483647 { parse_error(buf,i,"State numbers must be in the range 0 - 2147483647.") }
            Node::State(n as u32)
        },
        Some('L') => {
            expect_str(buf,i,"ist");
            Node::Coll(LIST_TYP, parse_tag(buf,i), parse_coll(buf,i))
        }
        Some('S') => {
            expect_str(buf,i,"et");
            Node::Coll(SET_TYP, parse_tag(buf,i), parse_coll(buf,i))
        },
        Some('A') => {
            expect_str(buf,i,"dd");
            Node::Mon(ADD_TYP, parse_tag(buf,i), parse_mon(buf,i))
        },
        Some('M') => {
            expect_str(buf,i,"ax");
            Node::Mon(MAX_TYP, parse_tag(buf,i), parse_mon(buf,i))
        },
        Some('O') => {
            expect_str(buf,i,"r");
            Node::Mon(OR_TYP, parse_tag(buf,i), parse_mon(buf,i))
        },
        _ => {
            parse_error(buf,i, "Expected the start of a node.")
        }
    }
}
fn parse_node_string(input: &str) -> Node {
    let mut i:usize = 0;
    let chrs = input.chars().collect::<Vec<_>>();
    let n = parse_node(&chrs, &mut i);
    if i+1 < chrs.len() {
        panic!("Did not parse whole input.");
    }
    return n
}

fn get_w(buf: &[u32], i: &mut usize) -> u32 {
    if *i >= buf.len() {
        panic!("Trying to parse past end of buffer.")
    }
    let w = buf[*i];
    *i += 1;
    return w;
}
fn parse_node_bin(buf: &[u32], i: &mut usize) -> Node {
    let w = get_w(buf,i);
    if is_state(w) {
        return Node::State(w)
    } else {
        let typ = get_typ(w);
        let tag = get_tag(w);
        let len = get_len(w);
        if typ == LIST_TYP || typ == SET_TYP {
            let mut ns:Vec<Node> = vec![];
            for _ in 0..len {
                ns.push(parse_node_bin(buf,i));
            }
            return Node::Coll(typ, tag, ns)
        }
        else if typ == ADD_TYP || typ == MAX_TYP || typ == OR_TYP {
            let mut ns = vec![];
            for _ in 0..len {
                let n = parse_node_bin(buf,i);
                let x1 = get_w(buf,i);
                let x2 = get_w(buf,i);
                ns.push((n, x1 as u64 | ((x2 as u64) << 32)));
            }
            return Node::Mon(typ, tag, ns)
        } else { panic!("Wrong typ in binary rep.") }
    }
}


#[test]
fn test_node_to_string(){
    let n = Node::Coll(1,2,
        vec![Node::State(32),Node::State(43),
        Node::Mon(ADD_TYP,4,vec![(Node::State(87),54),(Node::State(87),54)])]);
    let mut out: String = String::from("");
    node_to_string(n, &mut out);
    assert_eq!(out, "Set[2]{@32,@43,Add[4]{@87:54,@87:54}}")
}

#[test]
fn test_parse_node() {
    fn test_parse_node12(inp : &str, expected: &str) {
        // Parse inp then convert back to string and check if they're equal
        let n = parse_node_string(inp);
        let mut out: String = String::from("");
        node_to_string(n, &mut out);
        assert_eq!(out, expected);

        // Parse inp then convert to binary then parse binary then back
        // to string and check if they're equal
        let n = parse_node_string(inp);
        let mut outb: Vec<u32> = vec![];
        node_to_bin(&n,&mut outb);
        let mut i = 0;
        let n2 = parse_node_bin(&outb, &mut i);
        assert_eq!(i,outb.len());

        let mut out: String = String::from("");
        node_to_string(n2, &mut out);
        assert_eq!(out, expected);
    }
    fn test_parse_node1(inp : &str) {
        test_parse_node12(inp,inp)
    }
    test_parse_node1("Set[2]{@32,@43,Add[4]{@87:54,@87:54}}");
    test_parse_node1("List[2]{@32,@43,Add[4]{@87:54,@87:54}}");
    test_parse_node1("List[2]{@32,@43,Add[4]{@87:54,@87:54}}");
    test_parse_node1("List[2]{@32,@43,Max[4]{@87:54,@87:54}}");
    test_parse_node1("List[2]{@32,@43,Or[4]{@87:54,@87:54}}");
    test_parse_node1("List[2]{}");
    test_parse_node1("@345");
}



// Partition refinement
// ====================

type Loc = usize;
type State = u32;

struct Coalg {
    data: Vec<u32>, // binary representation of the coalgebra
    locs: Vec<Loc>, // locs[i] gives the index into data[loc[i]] where the i-th state starts
    backrefs: Vec<u32>, // buffer of backrefs
    backrefs_locs: Vec<u32> // backrefs_locs[i] gives the index into backrefs[backrefs_locs[i]] where the backrefs of the i-th state start
}

impl Coalg {
    fn new(data: Vec<u32>) -> Coalg {
        // TODO: make backrefs more efficient by not inserting duplicates if state i refers to state j twice

        // Iterate over one state starting at data[loc], calling f(i) on each state ref @i in the state.
        #[inline]
        fn iter<F>(data: &[u32], loc: &mut usize, f : &mut F)
        where F : FnMut(State) -> () {
            let w = data[*loc];
            if is_state(w) {
                f(w);
                *loc += 1;
            } else {
                let typ = get_typ(w);
                let len = get_len(w);
                *loc += 1;
                match typ {
                    LIST_TYP|SET_TYP => {
                        for _ in 0..len {
                            iter(data,loc,f)
                        }
                    },
                    ADD_TYP|MAX_TYP|OR_TYP => {
                        for _ in 0..len {
                            iter(data,loc,f);
                            *loc += 2;
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

        // Compute number of backrefs to state i in backrefs_locs[i]
        let mut loc = 0;
        let mut state_num:u32 = 0;
        while loc < data.len() {
            locs.push(loc);
            iter(&data, &mut loc, &mut |w| {
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
        loc = 0;
        let mut state_num:u32 = 0;
        while loc < data.len() {
            iter(&data, &mut loc, &mut |w| {
                // state_num refers to state w
                backrefs_locs[w as usize] -= 1;
                backrefs[backrefs_locs[w as usize] as usize] = state_num;
            });
            state_num += 1;
        }

        debug_assert_eq!(backrefs_locs.len(), state_num as usize + 1);

        Coalg {
            data: data,
            locs: locs,
            backrefs: backrefs,
            backrefs_locs: backrefs_locs
        }
    }

    fn state_backrefs(self: &Self, state: u32) -> &[u32] {
        let start = self.backrefs_locs[state as usize];
        let end = self.backrefs_locs[state as usize + 1];
        return &self.backrefs[start as usize..end as usize];
    }

    fn num_states(self: &Self) -> u32 {
        return self.locs.len() as u32
    }

    fn dump(self: &Self) {
        println!("Coalg {{\n  data: {:?},\n  locs: {:?},\n  backrefs: {:?},\n  backrefs_locs: {:?}\n}}", self.data, self.locs, self.backrefs, self.backrefs_locs);
    }

    fn dump_backrefs(self: &Self) {
        for state in 0..self.num_states() {
            println!("@{} backrefs={:?}", state, self.state_backrefs(state));
        }
    }
}

#[test]
fn test_new_coalg() {
    let data = read_boa_txt("tests/test1.boa.txt");
    // 0: List[0]{@0,@1}
    // 1: List[0]{@1,@1}
    // 2: List[1]{@0,@0}
    // 3: List[1]{@0,@0}
    // 4: List[1]{@3,@4}
    // 5: Add[0]{@0:1,@1:1}
    // 6: Add[0]{@0:2}
    // 7: Add[0]{@0:2,@1:1}
    let coa = Coalg::new(data);
    assert_eq!(coa.num_states(), 8);
    assert_eq!(&coa.backrefs, &vec![7,6,5,3,3,2,2,0,  7,5,1,1,0,  4,  4]); // 0,2,2,3,3,5,6,7,  0,1,1,5,7,  4,  4
    assert_eq!(&coa.backrefs_locs, &vec![0,8,13,13,14,15,15,15,15]); // note that the states 5,6,7 have no corresponding entry because they are never referred to and are at the end
    assert_eq!(&coa.state_backrefs(0), &vec![7,6,5,3,3,2,2,0]);
}

type ID = u32; // represents canonical ID of a state or sub-node of a state

struct Tables {
    last_id : ID,
    coll_table : HMap<Vec<ID>, ID>,
    mon_table : HMap<Vec<(ID,u64)>, ID>,
}

fn insert_or_op<A,F>(xs: &mut Vec<(A,u64)>, key: A, val: u64, op : F)
where F : Fn(u64,u64) -> u64, A:Ord {
    let r = xs.binary_search_by(|(key2,_)| key2.cmp(&key));
    match r {
        Ok(i) => {
            xs[i].1 = op(xs[i].1, val);
        }
        Err(i) => {
            xs.insert(i,(key,val));
        }
    }
}

#[test]
fn test_insert_or_op() {
    let mut xs = vec![];
    insert_or_op(&mut xs, 0, 1, |a,b| a+b);
    assert_eq!(xs, vec![(0,1)]);
    insert_or_op(&mut xs, 0, 1, |a,b| a+b);
    assert_eq!(xs, vec![(0,2)]);
    insert_or_op(&mut xs, 3, 1, |a,b| a+b);
    assert_eq!(xs, vec![(0,2),(3,1)]);
    insert_or_op(&mut xs, 2, 1, |a,b| a+b);
    assert_eq!(xs, vec![(0,2),(2,1),(3,1)]);
    insert_or_op(&mut xs, 2, 1, |a,b| a+b);
    assert_eq!(xs, vec![(0,2),(2,2),(3,1)]);
}

fn canonicalize(data : &[u32], ids: &[ID], loc : &mut Loc, tables : &mut Tables) -> ID {
    let w = data[*loc];
    if is_state(w) {
        *loc += 1;
        return ids[w as Loc]
    } else {
        let typ = get_typ(w);
        let tag = get_tag(w);
        let len = get_len(w);
        *loc += 1;
        match typ {
            LIST_TYP => {
                let mut children = vec![tag as ID];
                for _ in 0..len {
                    children.push(canonicalize(data, ids, loc,tables));
                }
                if tables.coll_table.contains_key(&children) {
                    return tables.coll_table[&children];
                } else {
                    let id = tables.last_id;
                    tables.last_id += 1;
                    tables.coll_table.insert(children, id);
                    return id
                }
            },
            SET_TYP => {
                let mut children = vec![];
                for _ in 0..len {
                    children.push(canonicalize(data, ids, loc, tables));
                }
                children.sort();
                children.dedup();
                children.push(tag as ID);
                if tables.coll_table.contains_key(&children) {
                    return tables.coll_table[&children];
                } else {
                    let id = tables.last_id;
                    tables.last_id += 1;
                    tables.coll_table.insert(children, id);
                    return id
                }
            },
            ADD_TYP => {
                let mut repr = vec![];
                for _ in 0..len {
                    let n = canonicalize(data, ids, loc, tables);
                    let x1 = data[*loc];
                    let x2 = data[*loc+1];
                    *loc += 2;
                    let w = x1 as u64 | ((x2 as u64) << 32);
                    insert_or_op(&mut repr, n, w, |a,b| a+b);
                }
                repr.push((tag as ID,0));
                if tables.mon_table.contains_key(&repr) {
                    return tables.mon_table[&repr];
                } else {
                    let id = tables.last_id;
                    tables.last_id += 1;
                    tables.mon_table.insert(repr, id);
                    return id
                }
            },
            MAX_TYP => {
                let mut repr = vec![];
                for _ in 0..len {
                    let n = canonicalize(data, ids, loc, tables);
                    let x1 = data[*loc];
                    let x2 = data[*loc+1];
                    *loc += 2;
                    let w = x1 as u64 | ((x2 as u64) << 32);
                    insert_or_op(&mut repr, n, w, |a,b| max(a,b));
                }
                repr.push((tag as ID,0));
                if tables.mon_table.contains_key(&repr) {
                    return tables.mon_table[&repr];
                } else {
                    let id = tables.last_id;
                    tables.last_id += 1;
                    tables.mon_table.insert(repr, id);
                    return id
                }
            },
            OR_TYP => {
                let mut repr = vec![];
                for _ in 0..len {
                    let n = canonicalize(data, ids, loc, tables);
                    let x1 = data[*loc];
                    let x2 = data[*loc+1];
                    *loc += 2;
                    let w = x1 as u64 | ((x2 as u64) << 32);
                    insert_or_op(&mut repr, n, w, |a,b| a|b);
                }
                repr.push((tag as ID,0));
                if tables.mon_table.contains_key(&repr) {
                    return tables.mon_table[&repr];
                } else {
                    let id = tables.last_id;
                    tables.last_id += 1;
                    tables.mon_table.insert(repr, id);
                    return id
                }
            },
            _ => {
                panic!("Unknown typ.")
            }
        }
    }
}

#[test]
fn canonicalize_test () {
    let data = read_boa_txt("tests/test1.boa.txt");
    let mut tables = Tables {
        last_id: 0,
        coll_table: HMap::new(),
        mon_table: HMap::new()
    };
    let ids = vec![0,0,0,0];
    let mut loc = 0;
    let canon_id1 = canonicalize(&data, &ids, &mut loc, &mut tables);
    let canon_id2 = canonicalize(&data, &ids, &mut loc, &mut tables);
    let canon_id3 = canonicalize(&data, &ids, &mut loc, &mut tables);
    let canon_id4 = canonicalize(&data, &ids, &mut loc, &mut tables);
    assert_eq!(canon_id1, 0);
    assert_eq!(canon_id2, 0);
    assert_eq!(canon_id3, 1);
    assert_eq!(canon_id4, 1);
}

// Returns vector of new IDs for each state in states
// IDs are labeled 0 to n
fn repartition(coa : &Coalg, states: &[State], ids: &[ID]) -> Vec<ID> {
    let mut tables = Tables {
        last_id: 0,
        coll_table: HMap::new(),
        mon_table: HMap::new()
    };
    let mut new_ids_raw = vec![];
    for &state in states {
        let mut loc_mut = coa.locs[state as usize];
        new_ids_raw.push(canonicalize(&coa.data, ids, &mut loc_mut, &mut tables));
    }
    return renumber(&new_ids_raw);
}

#[inline]
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

fn canonicalize_inexact_node<'a>(mut data : &'a [u32], ids: &[ID], w: u32) -> (u64, &'a [u32]) {
    let typ = get_typ(w);
    let tag = get_tag(w);
    let len = get_len(w);
    let mut hasher = new_hasher();
    tag.hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_inexact(data, ids);
                sig.hash(&mut hasher);
                data = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact(data, ids);
                data = rest; sig
            }).collect();
            repr.sort_unstable();
            repr.dedup();
            repr.hash(&mut hasher);
            // for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact(data, ids);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| a+b);
        },
        MAX_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact(data, ids);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b));
        },
        OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact(data, ids);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| a|b);
        },
        _ => {
            panic!("Unknown typ.")
        }
    }
    return (hasher.finish(), data);
}

#[inline]
fn canonicalize_inexact<'a>(data : &'a [u32], ids: &[ID]) -> (u64, &'a [u32]) {
    let w = data[0];
    let data = &data[1..];
    if is_state(w) {
        return (ids[w as Loc] as u64, data);
    } else {
        return canonicalize_inexact_node(data, ids, w);
    }
}

fn repartition_inexact(coa : &Coalg, states: &[State], ids: &[ID]) -> Vec<u64> {
    let mut sigs = vec![];
    sigs.reserve(states.len());
    for &state in states {
        let loc = coa.locs[state as usize];
        let (sig,_rest) = canonicalize_inexact(&coa.data[loc..], ids);
        sigs.push(sig);
    }
    return sigs;
}


fn skip_state(data: &[u32], loc: &mut usize) {
    let w = data[*loc];
    if is_state(w) {
        *loc += 1;
    } else {
        let typ = get_typ(w);
        let len = get_len(w);
        *loc += 1;
        match typ {
            LIST_TYP|SET_TYP => {
                for _ in 0..len {
                    skip_state(data,loc)
                }
            },
            ADD_TYP|MAX_TYP|OR_TYP => {
                for _ in 0..len {
                    skip_state(data,loc);
                    *loc += 2;
                }
            },
            _ => {
                panic!("Unknown typ.")
            }
        }
    }
}

/// Compute the starting index of each state
fn all_locs(data: &[u32]) -> Vec<usize> {
    let mut locs = vec![];
    let mut loc = 0;
    while loc < data.len() {
        locs.push(loc);
        skip_state(data, &mut loc);
    }
    return locs
}

#[test]
fn test_repartition_all() {
    let data = read_boa_txt("tests/test1.boa.txt");
    let ids = repartition_all_inexact(&data, &vec![0,0,0,0,0,0,0,0]);
    assert_eq!(&ids, &vec![0,0,1,1,1,2,2,3]);
    let ids = repartition_all(&data, &vec![0,0,0,0,0,0,0,0]);
    assert_eq!(&ids, &vec![0,0,1,1,1,2,2,3]);
}

fn read_boa_txt<P>(filename: P) -> Vec<u32>
where P: AsRef<Path>, {
    let file = File::open(&filename).
        expect(&format!("Couldn't open file {:?}", filename.as_ref().display().to_string()));
    let reader = BufReader::new(file);
    let mut data : Vec<u32> = vec![];
    for line in reader.lines() {
        let node = parse_node_string(&line.expect("Bad line"));
        node_to_bin(&node,&mut data);
    }
    return data
}

fn renumber<A> (ids: &[A]) -> Vec<u32>
where A:Hash+Eq {
    let mut canon_map = HMap::new();
    let mut last_id = 0;
    return ids.iter().map(|id| {
        if canon_map.contains_key(&id) {
            canon_map[&id]
        } else {
            canon_map.insert(id, last_id);
            last_id += 1;
            last_id - 1
        }
    }).collect();
}

fn cumsum_mut(xs: &mut [u32]) {
    let mut sum = 0;
    for x in xs.iter_mut() {
        sum += *x;
        *x = sum;
    }
}

#[test]
fn test_cumsum_mut() {
    let mut xs = vec![2,3,1,2,0,4];
    cumsum_mut(&mut xs);
    assert_eq!(xs, vec![2,5,6,8,8,12]);
}

fn cumsum(xs: &[u32]) -> Vec<u32> {
    let mut xs = xs.to_vec();
    cumsum_mut(&mut xs);
    return xs;
}

#[test]
fn test_cumsum() {
    let xs = vec![2,3,1,2,0,4];
    assert_eq!(cumsum(&xs), vec![2,5,6,8,8,12]);
}


fn repartition_all(data: &[u32], ids: &[ID]) -> Vec<ID> {
    let mut tables = Tables {
        last_id: 0,
        coll_table: HMap::new(),
        mon_table: HMap::new()
    };
    let mut new_ids_raw = vec![];
    let mut loc_mut = 0;
    while loc_mut < data.len() {
        new_ids_raw.push(canonicalize(data, ids, &mut loc_mut, &mut tables));
    }
    return renumber(&new_ids_raw)
}

fn repartition_all_inexact(data: &[u32], ids: &[ID]) -> Vec<ID> {
    let mut new_ids_raw = vec![];
    new_ids_raw.reserve(ids.len());
    let mut rest = data;
    while rest.len() > 0 {
        let (sig, rest_next) = canonicalize_inexact(rest, ids);
        new_ids_raw.push(sig);
        rest = rest_next;
    }
    return renumber(&new_ids_raw)
}

fn count_states(data: &[u32]) -> usize {
    let mut n = 0;
    let mut loc = 0;
    while loc < data.len() {
        n += 1;
        skip_state(data, &mut loc);
    }
    return n
}

fn canonicalize_inexact_node_init<'a>(mut data : &'a [u32], w: u32) -> (u64, &'a [u32]) {
    let typ = get_typ(w);
    let tag = get_tag(w);
    let len = get_len(w);
    let mut hasher = new_hasher();
    tag.hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_inexact_init(data);
                sig.hash(&mut hasher);
                data = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_init(data);
                data = rest; sig
            }).collect();
            repr.sort_unstable();
            repr.dedup();
            repr.hash(&mut hasher);
            // for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_init(data);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| a+b);
        },
        MAX_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_init(data);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b));
        },
        OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_init(data);
                let x1 = rest[0];
                let x2 = rest[1];
                data = &rest[2..];
                let w = x1 as u64 | ((x2 as u64) << 32);
                (sig,w)
            }).collect();
            hash_with_op(&mut repr, &mut hasher, |a,b| a|b);
        },
        _ => {
            panic!("Unknown typ.")
        }
    }
    return (hasher.finish(), data);
}

#[inline]
fn canonicalize_inexact_init<'a>(data : &'a [u32]) -> (u64, &'a [u32]) {
    let w = data[0];
    let data = &data[1..];
    if is_state(w) {
        return (0, data);
    } else {
        return canonicalize_inexact_node_init(data, w);
    }
}

fn init_partition_ids(data: &[u32]) -> Vec<u32> {
    let mut new_ids_raw = vec![];
    let mut rest = data;
    while rest.len() > 0 {
        let (sig, rest_next) = canonicalize_inexact_init(rest);
        new_ids_raw.push(sig);
        rest = rest_next;
    }
    return renumber(&new_ids_raw)
}

fn partref_naive(data: &[u32]) -> Vec<ID> {
    // let n = count_states(data);
    // let mut ids = vec![0;n];
    let mut ids = init_partition_ids(data);
    for iter in 0..1000000 {
        let start_time = SystemTime::now();
        let new_ids = repartition_all_inexact(data, &ids);
        let iter_time = start_time.elapsed().unwrap();

        // debug iteration info
        let mut new_ids2 = new_ids.clone();
        new_ids2.sort_unstable();
        new_ids2.dedup();
        let num_parts = new_ids2.len();
        // println!("- Iteration {}, number of partitions: {} (refinement time = {} seconds)", iter, num_parts, iter_time.as_secs_f32());
        // end debug info

        if new_ids[new_ids.len()-1] == new_ids.len() as u32 - 1 || new_ids == ids {
        // if new_ids == ids {
            println!("Number of iterations: {}", iter+1);
            return new_ids
        } else {
            ids = new_ids;
        }
    }
    panic!("Ran out of iterations.")
}

#[test]
fn test_partref_naive() {
    let data = read_boa_txt("tests/test1.boa.txt");
    let ids = partref_naive(&data);
    assert_eq!(&ids, &vec![0,0,1,1,2,3,3,4]);
}

fn counts_vec(xs: &[u32]) -> Vec<u32> {
    let mut counts = vec![];
    for &x in xs {
        while x as usize >= counts.len() { counts.push(0); }
        counts[x as usize] += 1;
    }
    return counts;
}

#[test]
fn test_counts_vec() {
    let counts = counts_vec(&vec![0,0,1,1,3,4,5,5,5]);
    assert_eq!(counts[0],2);
    assert_eq!(counts[1],2);
    assert_eq!(counts[3],1);
    assert_eq!(counts[4],1);
    assert_eq!(counts[5],3);
}

fn index_of_max(counts: &[u32]) -> usize {
    let mut i_max = usize::MAX;
    let mut v_max = 0;
    for i in 0..counts.len() {
        if counts[i] >= v_max {
            i_max = i;
            v_max = counts[i];
        }
    }
    return i_max
}

#[test]
fn test_index_of_max() {
    assert_eq!(index_of_max(&vec![0,3,1,2,3,4,3]), 5);
}

struct DirtyPartitions {
    buffer: Vec<State>, // buffer of states (partitioned)
    position: Vec<u32>, // position of each state in partition array
    partition_id: Vec<ID>, // partition of each state
    partition: Vec<(u32,u32,u32)>, // vector of partitions (start, mid, end) where the states in start..mid are mid and mid..end are clean
    worklist: Vec<u32>, // worklist of partitions
}

impl DirtyPartitions {
    fn new(num_states: u32) -> DirtyPartitions {
        DirtyPartitions {
            buffer: (0..num_states).collect(),
            position: (0..num_states).collect(),
            partition_id: vec![0;num_states as usize],
            partition: vec![(0, 0, num_states)], // for partition (start, mid, end), the states start..mid are clean and mid..end are dirty
            worklist: vec![0]
        }
    }

    /// Mark the state as dirty, putting its partition on the worklist if necessary
    /// Time complexity: O(1)
    fn mark_dirty(self: &mut DirtyPartitions, state: State) {
        let id = self.partition_id[state as usize];
        let pos = self.position[state as usize];
        let (start, mid, end) = self.partition[id as usize];
        // println!("mark_dirty(_,{}): id={}, pos={}, part={:?}", state, id, pos, (start,mid,end));
        if end - start <= 1 { return } // don't need to mark states dirty if they are in a singleton partition
        if mid <= pos { // state is already dirty
            return
        }
        if mid == end { // no dirty states in partition yet, so put it onto worklist
            self.worklist.push(id)
        }
        self.partition[id as usize].1 -= 1; // decrement the dirty states marker to make space
        let other_state = self.buffer[mid as usize - 1]; // the state that we will swap
        self.position[other_state as usize] = pos;
        self.position[state as usize] = mid;
        self.buffer[pos as usize] = other_state;
        self.buffer[mid as usize - 1] = state;
    }

    /// Determine slice of states to compute signatures for.
    /// Includes one clean state at the start if there are any clean states.
    /// Time complexity: O(1)
    fn refiners(self: &DirtyPartitions, id: ID) -> &[State] {
        let (start, mid, end) = self.partition[id as usize];
        if start == mid { // no clean states
            return &self.buffer[start as usize..end as usize]
        } else { // there are clean states
            return &self.buffer[(mid-1) as usize..end as usize]
        }
    }

    /// Time complexity: O(signatures.len())
    /// Returns vector of new partition ids
    /// Signatures are assumed to be 0..n with the first starting with 0
    fn refine(self: &mut DirtyPartitions, partition_id: ID, signatures: &[u32]) -> Vec<u32> {
        // let signatures = renumber(signatures); // Renumber signatures to be 0..n. This makes the sig of the clean states 0 if there are any.

        // compute the occurrence counts of each of the signatures
        let mut counts = counts_vec(&signatures);
        let (start,mid,end) = self.partition[partition_id as usize];
        if start < mid { counts[0] += mid - start - 1 } // add count of clean part

        // sort the relevant part of self.buffer by signature
        // also restores invariant for self.position and self.partition_id
        let largest_partition = index_of_max(&counts) as u32;
        let next_available_partition_id = self.partition.len() as u32;

        let mut cum_counts = cumsum(&counts);
        let original_states = self.refiners(partition_id).to_vec();
        for i in 0..original_states.len() {
            let sig = signatures[i];
            let state = original_states[i];
            cum_counts[sig as usize] -= 1;
            let j = start+cum_counts[sig as usize];
            self.buffer[j as usize] = state;
            self.position[state as usize] = j;

            if sig != largest_partition {
                let new_sig = next_available_partition_id + if sig < largest_partition { sig } else { sig - 1 };
                self.partition_id[state as usize] = new_sig;
            }
        }

        if largest_partition != 0 {
            // need to relabel the clean states
            for i in start..mid {
                let state = self.buffer[i as usize];
                self.partition_id[state as usize] = next_available_partition_id;
            }
        }

        if start < mid { cum_counts[0] -= mid - start - 1 }
        debug_assert_eq!(cum_counts[0],0);
        debug_assert_eq!(cum_counts[cum_counts.len()-1] + counts[counts.len()-1], end - start);


        // we will return vector of the new partitions
        let mut new_partitions: Vec<u32> = vec![];

        // restore invariant of self.partition
        for sig in 0..counts.len() as u32 {
            let new_start = start+cum_counts[sig as usize];
            let new_end = start+cum_counts[sig as usize]+counts[sig as usize];
            let new_part = (new_start, new_end, new_end); // all states are clean now (but may be marked dirty later)
            if sig == largest_partition {
                self.partition[partition_id as usize] = new_part;
            } else {
                new_partitions.push(self.partition.len() as u32);
                self.partition.push(new_part);
            }
        }

        return new_partitions;
    }
}

fn partref_nlogn_raw(data: Vec<u32>) -> Vec<ID> {
    // println!("===================== Starting partref_nlogn");
    // panic!("Stopped");
    let coa = Coalg::new(data);
    // coa.dump();
    // coa.dump_backrefs();
    let mut iters = 0;
    let mut parts = DirtyPartitions::new(coa.num_states());
    while let Some(partition_id) = parts.worklist.pop() {
        let states = parts.refiners(partition_id);
        // println!("states = {:?}", states);
        let signatures = renumber::<u64>(&repartition_inexact(&coa, states, &parts.partition_id));
        // println!("partition id = {:?}, partition = {:?}, states = {:?}, sigs = {:?}", partition_id, parts.partition[partition_id as usize], states, &signatures);
        let new_partitions = parts.refine(partition_id, &signatures);
        // println!("shrunk partition = {:?}, new partitions = {:?}, buffer = {:?}", parts.partition[partition_id as usize], &new_partitions.iter().map(|pid| parts.partition[*pid as usize]).collect::<Vec<(u32,u32,u32)>>(), &parts.buffer);
        for new_partition_id in new_partitions {
            // mark dirty all predecessors of states in this partition
            // let part_debug = parts.partition[new_partition_id as usize];
            let (start,_, end) = parts.partition[new_partition_id as usize];
            let states = parts.buffer[start as usize..end as usize].to_vec();
            for state in states {
                for &state2 in coa.state_backrefs(state) {
                    // println!("state {} marks state {} as dirty (new partition: {:?} id: {})", state, state2, &part_debug, new_partition_id);
                    parts.mark_dirty(state2);
                }
            }
        }
        iters += 1;
        // if iters > 47730 { panic!("Stop!") }
    }
    println!("Number of iterations: {} ", iters);
    // println!("===================== Ending partref_nlogn");
    return parts.partition_id;
}

fn partref_nlogn(data: Vec<u32>) -> Vec<ID> {
    let ids = partref_nlogn_raw(data);
    return renumber(&ids);
}

#[test]
fn test_partref_nlogn() {
    // List[0]{@0,@1}
    // List[0]{@1,@1}
    // List[1]{@0,@0}
    // List[1]{@0,@0}
    // List[1]{@3,@4}
    // Add[0]{@0:1,@1:1}
    // Add[0]{@0:2}
    // Add[0]{@0:2,@1:1}
    let data = read_boa_txt("tests/test1.boa.txt");
    let ids1 = partref_naive(&data);
    let ids2 = partref_nlogn(data);
    assert_eq!(&ids1, &ids2);

    let data = read_boa_txt("tests/test2.boa.txt");
    let ids = partref_nlogn(data);
    assert_eq!(&ids, &vec![0,1,2,3,4,5]);
}

#[test]
fn test_partref_wlan() {
    let filename = "benchmarks/small/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_4.boa.txt";
    let data = read_boa_txt(&filename);
    let ids = partref_nlogn(data);
    assert_eq!(*ids.iter().max().unwrap(), 107864);

    let filename = "benchmarks/wlan/wlan1_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1408676_1963522_roundrobin_32.boa.txt";
    let data = read_boa_txt(&filename);
    let ids = partref_nlogn(data);
    assert_eq!(*ids.iter().max().unwrap(), 243324);

    // let filename = "benchmarks/small/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_4.boa.txt";
    // let data = read_boa_txt(&filename);
    // let ids = partref_naive(&data);
    // assert_eq!(*ids.iter().max().unwrap(), 107864);

    // let filename = "benchmarks/wlan/wlan1_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1408676_1963522_roundrobin_32.boa.txt";
    // let data = read_boa_txt(&filename);
    // let ids = partref_naive(&data);
    // assert_eq!(*ids.iter().max().unwrap(), 243324);
}

fn main() -> Result<(),()> {
    let args:Vec<String> = env::args().collect();
    // println!("args: {:?}", &args);

    // let filename = "tests/test1.boa.txt";
    // let filename = "benchmarks/wlan/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_32.boa.txt";
    // let filename = "benchmarks/fms/fms.sm_n=4_35910_237120_roundrobin_32.boa.txt";
    // let filename = "benchmarks/small/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_4.boa.txt"; // 248502 106472
    // let filename = "benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.boa.txt";
    // let filename = "benchmarks/wlan/wlan2_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1632799_5456481_roundrobin_32.boa.txt";
    let filename = "benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.boa.txt";
    // let filename = "benchmarks/large/wta_Z,max_0,0,0,4_50399500_50_1007990_32.boa.txt";

    let filename =
        if args.len() > 1 { &args[1] }
        else { filename };

    let mut start_time = SystemTime::now();
    println!("Starting parsing {}... ", filename);
    let data = read_boa_txt(filename);
    let parsing_time = start_time.elapsed().unwrap();
    println!("Parsing done, size: {} in {} seconds", data.len(), parsing_time.as_secs_f32());

    start_time = SystemTime::now();
    let ids = partref_naive(&data);
    // let ids = partref_nlogn(data);
    println!("Number of states: {}, Number of partitions: {}", ids.len(), ids.iter().max().unwrap()+1);
    let computation_time = start_time.elapsed().unwrap();
    println!("Computation took {} seconds", computation_time.as_secs_f32());

    // let coa = Coalg::new(data);
    // println!("Number of states: {}", coa.num_states());

    // let mut i = 0;
    // for id in ids {
    //     println!("{}: {}", i, id);
    //     i += 1;
    // }
    // for file_res in glob::glob("benchmarks/*/*.boa.txt").unwrap() {
    //     let file = file_res.unwrap();
    //     let data = read_boa_txt(&file);
    //     println!("{:?}: {} Mb", file, (data.len()*4) as f64 / 1_000_000.0);
    // }

    Ok(())
}

