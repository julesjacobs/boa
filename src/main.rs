#![allow(dead_code)]
use std::{hash::{Hash, Hasher}, fs::File, io::{BufReader, BufRead, BufWriter, Write, Read}, path::Path, cmp::max, time::SystemTime, env};
use byteorder::{LittleEndian,WriteBytesExt, ReadBytesExt};
use datasize::{DataSize, data_size};
use itertools::Itertools;

fn mb(num_bytes: usize) -> String {
    let bytes_in_mb = ((1 as usize) << 20) as f64;
    let num_mb = num_bytes as f64 / bytes_in_mb;
    format!("{:.2} MB", num_mb)
}

//====================//
// Hasher & allocator //
//====================//

// FxHash appears to be the winner.
// Although AHash is a lot faster than the default hasher, I've found FxHash to be even faster.
// use fxhash::{FxHashMap, FxHasher64};
// fn new_hasher() -> FxHasher64 { FxHasher64::default() }
// type HMap<K,V> = FxHashMap<K,V>;

use ahash::{AHasher, AHashMap};
fn new_hasher() -> AHasher { AHasher::default() }
type HMap<K,V> = AHashMap<K,V>;


// Using a different allocator also makes a huge difference.
// I've found jemalloc to be better than mimalloc, both in terms of speed and memory use.
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use memmap::MmapOptions;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;


//=======================//
// Binary representation //
//=======================//

// The binary representation works as follows (everything is little endian).
// If the last bit of the first byte is 0, it is a single dictionary compressed byte (so we have a header dictionary with 128 entries)
// If the last two bits of the first byte are 01, the rest of the bits are a 4-byte state (thus we support up to 2^30 states).
// If the last two bits of the first byte are 11, it is a header.
// A header's first byte is its typ (indicating whether it is a list/set/add/or/max node).
// A header's second byte is its tag (just some additional data to distinguish states, e.g. different constructors of algebraic data type with the same length).
// A header's third and fourth byte are the len of the collection.
// For lists/sets, we then encode sequence of len states.
// For add/or/max, we then encode a sequence of len (state,value).
// Values are encoded as follows: if the last bit of the first byte is 0, then it is dictionary compressed (so we have a value dictionary with 128 entries).
// If the last bit of the first byte is 1, then the remaining bits encode the 63 bit value.


/// Compression tag
fn is_compressed32(w: u32) -> bool { w & 1 == 0 }
fn get_compressed32(w: u32) -> u8 { (w as u8) >> 1 }
fn get_noncompressed32(w: u32) -> u32 { w >> 1 }
fn put_noncompressed32(w: u32) -> u32 { (w << 1) | 1}

fn is_compressed64(w: u64) -> bool { w & 1 == 0 }
fn get_compressed64(w: u64) -> u8 { (w as u8) >> 1 }
fn get_noncompressed64(w: u64) -> u64 { w >> 1 }
fn put_noncompressed64(w: u64) -> u64 { (w << 1) | 1}

fn put_compressed8(w: u8) -> u8 { w << 1 }

/// Assumes that the compressed bit has already been removed
fn is_state(w: u32) -> bool { w & 1 == 0 }
fn get_state(w: u32) -> u32 { w >> 1 }
fn get_header(w: u32) -> u32 { w >> 1 }
fn put_state(w: u32) -> u32 { w << 1 }
fn put_header(w: u32) -> u32 { (w << 1) | 1 }

/// Assumes that the header tag has already been removed
fn decode_header(w: u32) -> (u8,u8,u16) { ((w >> 24) as u8, (w >> 16) as u8, w as u16) }
fn encode_header(typ: u8, tag: u8, len: u16) -> u32 { ((typ as u32) << 24) | ((tag as u32) << 16) | (len as u32) }

const LIST_TYP: u8 = 0;
const SET_TYP: u8 = 1;
const ADD_TYP: u8 = 2;
const MAX_TYP: u8 = 3;
const OR_TYP: u8 = 4;

#[test]
fn test_binary_representation() {
    assert_eq!(decode_header(encode_header(1,2,3)), (1,2,3));
    assert_eq!(decode_header(encode_header(1,u8::MAX,u16::MAX)), (1,u8::MAX,u16::MAX));
    assert_eq!(decode_header(get_header(
                 put_header(encode_header(127,u8::MAX,u16::MAX)))),
               (127,u8::MAX,u16::MAX));
    assert_eq!(decode_header(get_header(get_noncompressed32(
                put_noncompressed32(put_header(encode_header(63,u8::MAX,u16::MAX)))))),
              (63,u8::MAX,u16::MAX));
}


//=========================================//
// Dictionary compressed readers & writers //
//=========================================//

#[derive(DataSize)]
struct CReader {
    headers: [u32;128],
    values: [u64;128],
}

impl CReader {
    unsafe fn read_node(self: &Self, data: *const u8) -> (u32, *const u8) {
        let x = *(data as *const u32);
        if is_compressed32(x) { (self.headers[get_compressed32(x) as usize], data.add(1)) }
        else { (get_noncompressed32(x), data.add(4)) }
    }

    unsafe fn read_value(self: &Self, data: *const u8) -> (u64, *const u8) {
        let x = *(data as *const u64);
        if is_compressed64(x) { (self.values[get_compressed64(x) as usize], data.add(1)) }
        else { (get_noncompressed64(x), data.add(8)) }
    }

    unsafe fn read_node_mut(self: &Self, data: &mut *const u8) -> u32 {
        let (x,data2) = self.read_node(*data);
        *data = data2;
        return x;
    }

    unsafe fn read_value_mut(self: &Self, data: &mut *const u8) -> u64 {
        let (x,data2) = self.read_value(*data);
        *data = data2;
        return x;
    }

    unsafe fn is_at_end(data: &[u8], p: *const u8) -> bool {
        return data.as_ptr().add(data.len()) == p
    }
}

struct CWriter {
    headers_map: HMap<u32,u8>,
    values_map: HMap<u64,u8>,
    headers: [u32;128],
    values: [u64;128],
    data: Vec<u8>,
}

impl CWriter {
    fn new() -> CWriter {
        CWriter {
            headers_map: HMap::default(),
            values_map: HMap::default(),
            headers: [0;128],
            values: [0;128],
            data: vec![]
        }
    }

    fn finish(mut self: Self) -> (Vec<u8>, CReader) {
        self.data.reserve(7); // make sure to not trigger undefined behaviour by reading u64 at the last byte
        return (self.data, CReader {
            headers: self.headers,
            values: self.values,
        })
    }

    fn write_node(self: &mut Self, node: u32) {
        if self.headers_map.contains_key(&node) {
            self.data.push(self.headers_map[&node])
        } else {
            if self.headers_map.len() < 128 {
                let i = self.headers_map.len() as u8;
                self.headers_map.insert(node, put_compressed8(i));
                self.headers[i as usize] = node;
                self.data.push(put_compressed8(i));
            } else {
                // self.headers_map.insert(node, 255);
                // println!("Headers map size: {}", self.headers_map.len());
                // panic!("Node dict full");
                self.data.extend(u32::to_ne_bytes(put_noncompressed32(node)))
            }
        }
    }

    fn write_node_noncompressed(self: &mut Self, node: u32) {
        self.data.extend(u32::to_ne_bytes(put_noncompressed32(node)))
    }

    fn write_value(self: &mut Self, value: u64) {
        if self.values_map.contains_key(&value) {
            self.data.push(self.values_map[&value])
        } else {
            if self.values_map.len() < 128 {
                let i = self.values_map.len() as u8;
                self.values_map.insert(value, put_compressed8(i));
                self.values[i as usize] = value;
                self.data.push(put_compressed8(i));
            } else {
                // panic!("Value dict full");
                self.data.extend(u64::to_ne_bytes(put_noncompressed64(value)))
            }
        }
    }
}

#[test]
fn test_creader_cwriter() {
    let mut w = CWriter::new();
    for _ in 0..10 {
        for i in 0..1000 { w.write_node(i) }
        for i in 0..1000 { w.write_value(i) }
    }
    let (data, r) = w.finish();
    assert_eq!(data.len(), 10*(128 + (1000-128)*4 + 128 + (1000-128)*8));
    let mut p = data.as_ptr();
    unsafe {
        for _ in 0..10 {
            for i in 0..1000 { assert_eq!(r.read_node_mut(&mut p), i) }
            for i in 0..1000 { assert_eq!(r.read_value_mut(&mut p), i) }
        }
    }
}


//=================================//
// Convert between text and binary //
//=================================//

#[derive(PartialEq)]
#[derive(Debug)]
pub enum Node {
    State(u32),
    Coll(u8,u8,Vec<Node>),
    Mon(u8,u8,Vec<(Node,u64)>)
}

mod parsing {
    use crate::*;

    fn read_expect<'a>(inp: &'a [u8], chr: u8) -> &'a [u8] {
        if inp.len() == 0 || inp[0] != chr {
            panic!("Expecting {:?}, got {:?}.", chr as char, String::from_utf8(inp.to_vec()).unwrap());
        }
        return &inp[1..];
    }

    fn read_tag<'a>(inp: &'a [u8]) -> (u8, &'a [u8]) {
        let inp = read_expect(inp, b'[');
        let (tag,n) = lexical::parse_partial::<u8,_>(inp).expect("Expected a number in tag [_].");
        (tag, read_expect(&inp[n..], b']'))
    }

    #[test]
    fn test_read_tag() {
        assert_eq!(read_tag("[123]abc".as_bytes()), (123, "abc".as_bytes()));
    }

    fn read_coll<'a>(inp: &'a [u8], typ: u8) -> (Node, &'a [u8]) {
        let (tag, inp) = read_tag(inp);
        let mut inp = read_expect(inp, b'{');
        let mut nodes = vec![];
        if inp.len() == 0 { panic!("Unexpected end of input at start of collection.") }
        if inp[0] == b'}' { return (Node::Coll(typ, tag, nodes), &inp[1..]) }
        loop {
            let (node,inp2) = read_node(inp);
            inp = inp2;
            nodes.push(node);
            if inp.len() == 0 { panic!("Unexpected end of input in collection.") }
            if inp[0] == b'}' { return (Node::Coll(typ, tag, nodes), &inp[1..]) }
            inp = read_expect(inp, b',');
        }
    }

    #[test]
    fn test_read_coll() {
        assert_eq!(read_coll("[123]{@12,@13,@14}abc".as_bytes(), LIST_TYP),
                (Node::Coll(LIST_TYP, 123, vec![Node::State(12),Node::State(13),Node::State(14)]),"abc".as_bytes()));
    }

    fn read_mon<'a>(inp: &'a [u8], typ: u8) -> (Node, &'a [u8]) {
        let (tag, inp) = read_tag(inp);
        let mut inp = read_expect(inp, b'{');
        let mut nodes = vec![];
        if inp.len() == 0 { panic!("Unexpected end of input at start of monoid.") }
        if inp[0] == b'}' { return (Node::Mon(typ, tag, nodes), &inp[1..]) }
        loop {
            let (node,inp2) = read_node(inp);
            inp = read_expect(inp2, b':');
            let (val,n) = lexical::parse_partial::<u64,_>(inp).expect("Expected a number after ':'.");
            inp = &inp[n..];
            nodes.push((node, val));
            if inp.len() == 0 { panic!("Unexpected end of input in monoid.") }
            if inp[0] == b'}' { return (Node::Mon(typ, tag, nodes), &inp[1..]) }
            inp = read_expect(inp, b',');
        }
    }

    #[test]
    fn test_read_mon() {
        assert_eq!(read_mon("[123]{@12:5,@13:6,@14:7}abc".as_bytes(), ADD_TYP),
            (Node::Mon(ADD_TYP, 123, vec![(Node::State(12),5),(Node::State(13),6),(Node::State(14),7)]),"abc".as_bytes()));
    }

    pub fn read_node<'a>(inp: &'a [u8]) -> (Node, &'a [u8]) {
        if inp.len() == 0 { panic!("Expected start of a node, but input is empty.") }
        let chr = inp[0];
        let orig = inp;
        let inp = &inp[1..];
        match chr {
            b'@' => {
                let (state,n) = lexical::parse_partial::<u32,_>(inp).expect("Expected a number after '@'.");
                assert!(state <= u32::MAX >> 2);
                (Node::State(state), &inp[n..])
            },
            b'L' => {
                if inp.len() < 3 || inp[0..3] != [b'i', b's', b't'] {
                    panic!("Expected \"List\", got {:?}", String::from_utf8(orig.to_vec()).unwrap());
                }
                return read_coll(&inp[3..], LIST_TYP);
            },
            b'S' => {
                if inp.len() < 2 || inp[0..2] != [b'e', b't'] {
                    panic!("Expected \"Set\" got {:?}", String::from_utf8(orig.to_vec()).unwrap());
                }
                return read_coll(&inp[2..], SET_TYP);
            },
            b'A' => {
                if inp.len() < 2 || inp[0..2] != [b'd', b'd'] {
                    panic!("Expected \"Add\" got {:?}", String::from_utf8(orig.to_vec()).unwrap());
                }
                return read_mon(&inp[2..], ADD_TYP);
            },
            b'O' => {
                if inp.len() < 1 || inp[0..1] != [b'r'] {
                    panic!("Expected \"Or\" got {:?}", String::from_utf8(orig.to_vec()).unwrap());
                }
                return read_mon(&inp[1..], OR_TYP);
            },
            b'M' => {
                if inp.len() < 2 || inp[0..2] != [b'a', b'x'] {
                    panic!("Expected \"Max\", got {:?}", String::from_utf8(orig.to_vec()).unwrap());
                }
                return read_mon(&inp[2..], MAX_TYP);
            },
            _ => { panic!("Expected start of a node, but got {:?}.", String::from_utf8(orig.to_vec()).unwrap()) }
        }
    }

    #[test]
    fn test_read_node() {
        assert_eq!(read_node("List[123]{@12,@13,@14}abc".as_bytes()),
            (Node::Coll(LIST_TYP, 123, vec![Node::State(12),Node::State(13),Node::State(14)]), "abc".as_bytes()));

        assert_eq!(read_node("Set[123]{@12,@13,@14}abc".as_bytes()),
            (Node::Coll(SET_TYP, 123, vec![Node::State(12),Node::State(13),Node::State(14)]), "abc".as_bytes()));

        assert_eq!(read_node("Set[123]{}abc".as_bytes()),
            (Node::Coll(SET_TYP, 123, vec![]), "abc".as_bytes()));

        assert_eq!(read_node("Add[123]{@12:5,@13:6,@14:7}abc".as_bytes()),
            (Node::Mon(ADD_TYP, 123, vec![(Node::State(12),5),(Node::State(13),6),(Node::State(14),7)]),"abc".as_bytes()));

        assert_eq!(read_node("Or[123]{@12:5,@13:6,@14:7}abc".as_bytes()),
            (Node::Mon(OR_TYP, 123, vec![(Node::State(12),5),(Node::State(13),6),(Node::State(14),7)]),"abc".as_bytes()));

        assert_eq!(read_node("Max[123]{@12:5,@13:6,@14:7}abc".as_bytes()),
            (Node::Mon(MAX_TYP, 123, vec![(Node::State(12),5),(Node::State(13),6),(Node::State(14),7)]),"abc".as_bytes()));

        assert_eq!(read_node("Max[123]{}abc".as_bytes()),
            (Node::Mon(MAX_TYP, 123, vec![]),"abc".as_bytes()));
    }
}

impl Node {
    fn from_ascii(inp: &[u8]) -> Self {
        let (node,rest) = parsing::read_node(inp);
        if rest.len() == 0 || rest == [b'\n'] {
            return node
        } else {
            panic!("Did not parse everything on the line.")
        }
    }

    fn to_ascii(self: &Self, w: &mut Vec<u8>) {
        match self {
            Node::State(state) => {
                w.push(b'@');
                w.extend(lexical::to_string(*state).as_bytes());
            }
            Node::Coll(typ, tag, nodes) => {
                let typ_str = match *typ {
                    LIST_TYP => "List[",
                    SET_TYP => "Set[",
                    _ => panic!("Bad typ.")
                };
                w.extend(typ_str.as_bytes());
                w.extend(lexical::to_string(*tag).as_bytes());
                w.extend([b']',b'{']);
                for node in nodes {
                    node.to_ascii(w);
                    w.push(b',');
                }
                if nodes.len() > 0 { w.pop(); }
                w.push(b'}');
            }
            Node::Mon(typ, tag, nodes) => {
                let typ_str = match *typ {
                    ADD_TYP => "Add[",
                    OR_TYP => "Or[",
                    MAX_TYP => "Max[",
                    _ => panic!("Bad typ.")
                };
                w.extend(typ_str.as_bytes());
                w.extend(lexical::to_string(*tag).as_bytes());
                w.extend([b']',b'{']);
                for (node,val) in nodes {
                    node.to_ascii(w);
                    w.push(b':');
                    w.extend(lexical::to_string(*val).as_bytes());
                    w.push(b',');
                }
                if nodes.len() > 0 { w.pop(); }
                w.push(b'}');
            }
        }
    }

    fn write(self: &Self, w: &mut CWriter) {
        match self {
            Node::State(state) => w.write_node_noncompressed(put_state(*state)),
            Node::Coll(typ, tag, nodes) => {
                w.write_node(put_header(encode_header(*typ, *tag, nodes.len() as u16)));
                for node in nodes { node.write(w) }
            }
            Node::Mon(typ, tag, nodes) => {
                w.write_node(put_header(encode_header(*typ, *tag, nodes.len() as u16)));
                for (node, val) in nodes { node.write(w); w.write_value(*val) }
            }
        }
    }

    unsafe fn read(r: &CReader, p: &mut *const u8) -> Self {
        let w = r.read_node_mut(p);
        if is_state(w) {
            Node::State(get_state(w))
        } else {
            let (typ, tag, len) = decode_header(get_header(w));
            match typ {
                LIST_TYP| SET_TYP => {
                    let nodes = (0..len).map(|_| { Node::read(r, p) }).collect();
                    Node::Coll(typ, tag, nodes)
                },
                ADD_TYP| OR_TYP| MAX_TYP => {
                    let nodes = (0..len).map(|_| {
                        let node = Node::read(r, p);
                        let val = r.read_value_mut(p);
                        (node,val)
                    }).collect();
                    Node::Mon(typ, tag, nodes)
                }
                _ => { panic!("Unknown typ.") }
            }
        }
    }
}

#[test]
fn test_node_read_write() {
    // Test conversion from & to ascii
    let node_str = "Max[123]{@12:1,Set[123]{@12,@13,@14}:2,Max[123]{@12:3,@13:4,@14:5}:6,Set[12]{}:7}";
    let node = Node::from_ascii(node_str.as_bytes());
    let mut out = vec![];
    node.to_ascii(&mut out);
    assert_eq!(String::from_utf8(out).unwrap(), node_str);

    // Test conversion to & from binary
    let mut w = CWriter::new();
    node.write(&mut w);
    let (data,r) = w.finish();
    unsafe {
        let node2 = Node::read(&r, &mut data.as_ptr());
        assert_eq!(node, node2);
    }
}

fn read_boa_txt<P>(filename: P) -> (Vec<u8>,CReader)
where P: AsRef<Path>, {
    let filename_str = filename.as_ref().display().to_string();
    if !filename_str.ends_with(".boa.txt") {
        panic!("File must be *.boa.txt, but is {}", filename_str);
    }
    let file = File::open(&filename).
        expect(&format!("Couldn't open file {:?}", filename.as_ref().display().to_string()));
    let mut reader = BufReader::new(file);
    let mut line = vec![];
    let mut w = CWriter::new();
    while 0 < reader.read_until(b'\n', &mut line).expect("Failure while reading file.") {
        let node = Node::from_ascii(&line);
        node.write(&mut w);
        line.clear();
    }
    w.finish()
}

fn create_file<P>(filename: P) -> File
where P: AsRef<Path>, {
    if filename.as_ref().exists() { panic!("File already exists: {:?}", filename.as_ref().display().to_string()) }
    let file = File::create(&filename).
        expect(&format!("Couldn't create file {:?}", filename.as_ref().display().to_string()));
    return file
}

fn write_boa_txt<P>(filename: P, data: &[u8], r: &CReader)
where P: AsRef<Path>, {
    let filename_str = filename.as_ref().display().to_string();
    if !filename_str.ends_with(".boa.txt") {
        panic!("File must be *.boa.txt, but is {}", filename_str);
    }
    let file = create_file(filename);
    let mut writer = BufWriter::new(file);
    let mut buf = vec![];
    unsafe {
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let node = Node::read(r, &mut p);
            node.to_ascii(&mut buf);
            if !CReader::is_at_end(data, p) { buf.push(b'\n') };
            writer.write_all(&buf);
            buf.clear();
        }
    }
}

fn read_boa<P>(filename: P) -> (Vec<u8>,CReader)
where P: AsRef<Path>, {
    let filename_str = filename.as_ref().display().to_string();
    if !filename_str.ends_with(".boa") {
        panic!("File must be *.boa, but is {}", filename_str);
    }
    let mut file = File::open(&filename).
        expect(&format!("Couldn't open file {:?}", filename.as_ref().display().to_string()));
    let mut r = CReader { headers: [0;128], values: [0;128] };
    for i in 0..r.headers.len() {
        r.headers[i] = file.read_u32::<LittleEndian>().expect("File reading error.");
    }
    for i in 0..r.values.len() {
        r.values[i] = file.read_u64::<LittleEndian>().expect("File reading error.");
    }
    let size = file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
    let mut data = Vec::with_capacity(size);
    file.read_to_end(&mut data).expect("File reading error.");
    return (data,r)
}


fn write_boa<P>(filename: P, data: &[u8], r: &CReader)
where P: AsRef<Path>, {
    let filename_str = filename.as_ref().display().to_string();
    if !filename_str.ends_with(".boa") {
        panic!("File must be *.boa, but is {}", filename_str);
    }
    let file = create_file(filename);
    let mut writer = BufWriter::new(file);
    for header in r.headers {
        writer.write_u32::<LittleEndian>(header).expect("Writing error.");
    }
    for value in r.values {
        writer.write_u64::<LittleEndian>(value).expect("Writing error.");;
    }
    writer.write_all(data).expect("Writing error.");;
}

fn convert_file(filename: &str) {
    if filename.ends_with(".boa") {
        let new_filename = [&filename[0..filename.len()-4],".boa.txt"].concat();
        let (data,r) = read_boa(filename);
        write_boa_txt(new_filename, &data, &r);
    } else if filename.ends_with(".boa.txt") {
        let new_filename = [&filename[0..filename.len()-8],".boa"].concat();
        let (data,r) = read_boa_txt(filename);
        write_boa(new_filename, &data, &r);
    } else {
        panic!("Unknown file type: {}", filename)
    }
}

#[test]
fn test_convert_file() {
    std::fs::remove_file("tests/test1_converted.boa.txt").unwrap();
    std::fs::remove_file("tests/test1.boa").unwrap();
    convert_file("tests/test1_converted.boa");
    convert_file("tests/test1.boa.txt");
}

//======================//
// Partition refinement //
//======================//

fn ptrvec_datasize(v: &Vec<*const u8>) -> usize { v.len() * 8 }

#[derive(DataSize)]
struct Coalg {
    data: Vec<u8>, // binary representation of the coalgebra
    reader: CReader,
    #[data_size(with = ptrvec_datasize)]
    locs: Vec<*const u8>, // gives the location in data where the i-th state starts
    backrefs: Vec<u32>, // buffer of backrefs
    backrefs_locs: Vec<u32> // backrefs_locs[i] gives the index into backrefs[backrefs_locs[i]] where the backrefs of the i-th state start
}


impl Coalg {
    fn new(data: Vec<u8>, r: CReader) -> Coalg {
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
                    ADD_TYP|MAX_TYP|OR_TYP => {
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
    let (data,r) = read_boa_txt("tests/test1.boa.txt");
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

// struct Tables {
//     last_id : ID,
//     coll_table : HMap<Vec<ID>, ID>,
//     mon_table : HMap<Vec<(ID,u64)>, ID>,
// }

// fn insert_or_op<A,F>(xs: &mut Vec<(A,u64)>, key: A, val: u64, op : F)
// where F : Fn(u64,u64) -> u64, A:Ord {
//     let r = xs.binary_search_by(|(key2,_)| key2.cmp(&key));
//     match r {
//         Ok(i) => {
//             xs[i].1 = op(xs[i].1, val);
//         }
//         Err(i) => {
//             xs.insert(i,(key,val));
//         }
//     }
// }

// #[test]
// fn test_insert_or_op() {
//     let mut xs = vec![];
//     insert_or_op(&mut xs, 0, 1, |a,b| a+b);
//     assert_eq!(xs, vec![(0,1)]);
//     insert_or_op(&mut xs, 0, 1, |a,b| a+b);
//     assert_eq!(xs, vec![(0,2)]);
//     insert_or_op(&mut xs, 3, 1, |a,b| a+b);
//     assert_eq!(xs, vec![(0,2),(3,1)]);
//     insert_or_op(&mut xs, 2, 1, |a,b| a+b);
//     assert_eq!(xs, vec![(0,2),(2,1),(3,1)]);
//     insert_or_op(&mut xs, 2, 1, |a,b| a+b);
//     assert_eq!(xs, vec![(0,2),(2,2),(3,1)]);
// }

// fn canonicalize(data : &[u32], ids: &[ID], loc : &mut Loc, tables : &mut Tables) -> ID {
//     let w = data[*loc];
//     if is_state(w) {
//         *loc += 1;
//         return ids[w as Loc]
//     } else {
//         let typ = get_typ(w);
//         let tag = get_tag(w);
//         let len = get_len(w);
//         *loc += 1;
//         match typ {
//             LIST_TYP => {
//                 let mut children = vec![tag as ID];
//                 for _ in 0..len {
//                     children.push(canonicalize(data, ids, loc,tables));
//                 }
//                 if tables.coll_table.contains_key(&children) {
//                     return tables.coll_table[&children];
//                 } else {
//                     let id = tables.last_id;
//                     tables.last_id += 1;
//                     tables.coll_table.insert(children, id);
//                     return id
//                 }
//             },
//             SET_TYP => {
//                 let mut children = vec![];
//                 for _ in 0..len {
//                     children.push(canonicalize(data, ids, loc, tables));
//                 }
//                 children.sort();
//                 children.dedup();
//                 children.push(tag as ID);
//                 if tables.coll_table.contains_key(&children) {
//                     return tables.coll_table[&children];
//                 } else {
//                     let id = tables.last_id;
//                     tables.last_id += 1;
//                     tables.coll_table.insert(children, id);
//                     return id
//                 }
//             },
//             ADD_TYP => {
//                 let mut repr = vec![];
//                 for _ in 0..len {
//                     let n = canonicalize(data, ids, loc, tables);
//                     let x1 = data[*loc];
//                     let x2 = data[*loc+1];
//                     *loc += 2;
//                     let w = x1 as u64 | ((x2 as u64) << 32);
//                     insert_or_op(&mut repr, n, w, |a,b| a+b);
//                 }
//                 repr.push((tag as ID,0));
//                 if tables.mon_table.contains_key(&repr) {
//                     return tables.mon_table[&repr];
//                 } else {
//                     let id = tables.last_id;
//                     tables.last_id += 1;
//                     tables.mon_table.insert(repr, id);
//                     return id
//                 }
//             },
//             MAX_TYP => {
//                 let mut repr = vec![];
//                 for _ in 0..len {
//                     let n = canonicalize(data, ids, loc, tables);
//                     let x1 = data[*loc];
//                     let x2 = data[*loc+1];
//                     *loc += 2;
//                     let w = x1 as u64 | ((x2 as u64) << 32);
//                     insert_or_op(&mut repr, n, w, |a,b| max(a,b));
//                 }
//                 repr.push((tag as ID,0));
//                 if tables.mon_table.contains_key(&repr) {
//                     return tables.mon_table[&repr];
//                 } else {
//                     let id = tables.last_id;
//                     tables.last_id += 1;
//                     tables.mon_table.insert(repr, id);
//                     return id
//                 }
//             },
//             OR_TYP => {
//                 let mut repr = vec![];
//                 for _ in 0..len {
//                     let n = canonicalize(data, ids, loc, tables);
//                     let x1 = data[*loc];
//                     let x2 = data[*loc+1];
//                     *loc += 2;
//                     let w = x1 as u64 | ((x2 as u64) << 32);
//                     insert_or_op(&mut repr, n, w, |a,b| a|b);
//                 }
//                 repr.push((tag as ID,0));
//                 if tables.mon_table.contains_key(&repr) {
//                     return tables.mon_table[&repr];
//                 } else {
//                     let id = tables.last_id;
//                     tables.last_id += 1;
//                     tables.mon_table.insert(repr, id);
//                     return id
//                 }
//             },
//             _ => {
//                 panic!("Unknown typ.")
//             }
//         }
//     }
// }

// #[test]
// fn canonicalize_test () {
//     let data = read_boa_txt("tests/test1.boa.txt");
//     let mut tables = Tables {
//         last_id: 0,
//         coll_table: HMap::default(),
//         mon_table: HMap::default()
//     };
//     let ids = vec![0,0,0,0];
//     let mut loc = 0;
//     let canon_id1 = canonicalize(&data, &ids, &mut loc, &mut tables);
//     let canon_id2 = canonicalize(&data, &ids, &mut loc, &mut tables);
//     let canon_id3 = canonicalize(&data, &ids, &mut loc, &mut tables);
//     let canon_id4 = canonicalize(&data, &ids, &mut loc, &mut tables);
//     assert_eq!(canon_id1, 0);
//     assert_eq!(canon_id2, 0);
//     assert_eq!(canon_id3, 1);
//     assert_eq!(canon_id4, 1);
// }

// // Returns vector of new IDs for each state in states
// // IDs are labeled 0 to n
// fn repartition(coa : &Coalg, states: &[State], ids: &[ID]) -> Vec<ID> {
//     let mut tables = Tables {
//         last_id: 0,
//         coll_table: HMap::default(),
//         mon_table: HMap::default()
//     };
//     let mut new_ids_raw = vec![];
//     for &state in states {
//         let mut loc_mut = coa.locs[state as usize];
//         new_ids_raw.push(canonicalize(&coa.data, ids, &mut loc_mut, &mut tables));
//     }
//     return renumber(&new_ids_raw);
// }

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

// fn canonicalize_inexact_node<'a>(mut data : &'a [u32], ids: &[ID], w: u32) -> (u64, &'a [u32]) {
//     let typ = get_typ(w);
//     let tag = get_tag(w);
//     let len = get_len(w);
//     let mut hasher = new_hasher();
//     tag.hash(&mut hasher);
//     match typ {
//         LIST_TYP => {
//             for _ in 0..len {
//                 let (sig, rest) = canonicalize_inexact(data, ids);
//                 sig.hash(&mut hasher);
//                 data = rest;
//             }
//         },
//         SET_TYP => {
//             let mut repr: Vec<u64> = (0..len).map(|_| {
//                 let (sig, rest) = canonicalize_inexact(data, ids);
//                 data = rest; sig
//             }).collect();
//             repr.sort_unstable();
//             repr.dedup();
//             repr.hash(&mut hasher);
//             // for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
//         },
//         ADD_TYP => {
//             let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
//                 let (sig, rest) = canonicalize_inexact(data, ids);
//                 let x1 = rest[0];
//                 let x2 = rest[1];
//                 data = &rest[2..];
//                 let w = x1 as u64 | ((x2 as u64) << 32);
//                 (sig,w)
//             }).collect();
//             hash_with_op(&mut repr, &mut hasher, |a,b| a+b);
//         },
//         MAX_TYP => {
//             let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
//                 let (sig, rest) = canonicalize_inexact(data, ids);
//                 let x1 = rest[0];
//                 let x2 = rest[1];
//                 data = &rest[2..];
//                 let w = x1 as u64 | ((x2 as u64) << 32);
//                 (sig,w)
//             }).collect();
//             hash_with_op(&mut repr, &mut hasher, |a,b| max(a,b));
//         },
//         OR_TYP => {
//             let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
//                 let (sig, rest) = canonicalize_inexact(data, ids);
//                 let x1 = rest[0];
//                 let x2 = rest[1];
//                 data = &rest[2..];
//                 let w = x1 as u64 | ((x2 as u64) << 32);
//                 (sig,w)
//             }).collect();
//             hash_with_op(&mut repr, &mut hasher, |a,b| a|b);
//         },
//         _ => {
//             panic!("Unknown typ.")
//         }
//     }
//     return (hasher.finish(), data);
// }

// #[inline]
// fn canonicalize_inexact<'a>(data : &'a [u32], ids: &[ID]) -> (u64, &'a [u32]) {
//     let w = data[0];
//     let data = &data[1..];
//     if is_state(w) {
//         return (ids[w as Loc] as u64, data);
//     } else {
//         return canonicalize_inexact_node(data, ids, w);
//     }
// }

// fn repartition_inexact(coa : &Coalg, states: &[State], ids: &[ID]) -> Vec<u64> {
//     let mut sigs = vec![];
//     sigs.reserve(states.len());
//     for &state in states {
//         let loc = coa.locs[state as usize];
//         let (sig,_rest) = canonicalize_inexact(&coa.data[loc..], ids);
//         sigs.push(sig);
//     }
//     return sigs;
// }

unsafe fn canonicalize_inexact_node_unsafe<'a>(mut p : *const u8, r: &CReader, ids: &[ID], w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_inexact_unsafe(p, r, ids);
                sig.hash(&mut hasher);
                p = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_unsafe(p, r, ids);
                p = rest; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_inexact_unsafe(p, r, ids);
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
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

unsafe fn canonicalize_inexact_unsafe<'a>(p : *const u8, r: &CReader, ids: &[ID]) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (ids[get_state(w) as usize] as u64, p);
    } else {
        return canonicalize_inexact_node_unsafe(p, r, ids, get_header(w));
    }
}

fn repartition_inexact_unsafe(coa : &Coalg, states: &[u32], ids: &[ID]) -> Vec<u64> {
    let mut sigs = vec![];
    sigs.reserve(states.len());
    for &state in states {
        let p = coa.locs[state as usize];
        unsafe {
            let (sig,_rest) = canonicalize_inexact_unsafe(p, &coa.reader, ids);
            sigs.push(sig);
        }
    }
    return sigs
}

unsafe fn canonicalize_inexact_node_unsafe64<'a>(mut p : *const u8, r: &CReader, ids: &[u64], w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, rest) = canonicalize_inexact_unsafe64(p, r, ids);
                sig.hash(&mut hasher);
                p = rest;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, rest) = canonicalize_inexact_unsafe64(p, r, ids);
                p = rest; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_inexact_unsafe64(p, r, ids);
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
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

unsafe fn canonicalize_inexact_unsafe64<'a>(p : *const u8, r: &CReader, ids: &[u64]) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (ids[get_state(w) as usize] as u64, p);
    } else {
        return canonicalize_inexact_node_unsafe64(p, r, ids, get_header(w));
    }
}

fn repartition_inexact_unsafe64(coa : &Coalg, states: &[u32], ids: &[u64]) -> Vec<u64> {
    let mut sigs = vec![];
    sigs.reserve(states.len());
    for &state in states {
        let p = coa.locs[state as usize];
        unsafe {
            let (sig,_rest) = canonicalize_inexact_unsafe64(p, &coa.reader, ids);
            sigs.push(sig);
        }
    }
    return sigs;
}

fn repartition_all_inexact_unsafe64(data: &[u8], r: &CReader, ids: &[u64]) -> Vec<u64> {
    unsafe {
        let mut new_ids_raw = vec![];
        new_ids_raw.reserve(ids.len());
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let (sig, p_next) = canonicalize_inexact_unsafe64(p, r, ids);
            new_ids_raw.push(sig);
            p = p_next;
        }
        return new_ids_raw
    }
}


unsafe fn canonicalize_inexact_node_unsafe_init<'a>(mut p : *const u8, r: &CReader, w: u32) -> (u64, *const u8) {
    let (typ,tag,len) = decode_header(w);
    let mut hasher = new_hasher();
    (typ,tag).hash(&mut hasher);
    match typ {
        LIST_TYP => {
            for _ in 0..len {
                let (sig, p2) = canonicalize_inexact_unsafe_init(p, r);
                sig.hash(&mut hasher);
                p = p2;
            }
        },
        SET_TYP => {
            let mut repr: Vec<u64> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_inexact_unsafe_init(p, r);
                p = p2; sig
            }).collect();
            repr.sort_unstable();
            for &sig in repr.iter().dedup() { sig.hash(&mut hasher); }
        },
        ADD_TYP|MAX_TYP|OR_TYP => {
            let mut repr: Vec<(u64,u64)> = (0..len).map(|_| {
                let (sig, p2) = canonicalize_inexact_unsafe_init(p, r);
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
        _ => panic!("Unknown typ.")
    }
    return (hasher.finish(), p);
}

unsafe fn canonicalize_inexact_unsafe_init<'a>(p : *const u8, r: &CReader) -> (u64, *const u8) {
    let (w,p) = r.read_node(p);
    if is_state(w) {
        return (0, p);
    } else {
        return canonicalize_inexact_node_unsafe_init(p, r, get_header(w));
    }
}

fn init_partition_ids_unsafe(data: &[u8], r: &CReader) -> Vec<u64> {
    unsafe {
        let mut new_ids_raw = vec![];
        let mut p = data.as_ptr();
        while !CReader::is_at_end(data, p) {
            let (sig, p_next) = canonicalize_inexact_unsafe_init(p, r);
            new_ids_raw.push(sig);
            p = p_next;
        }
        return new_ids_raw
    }
}

fn count_parts(sigs: &[u64]) -> usize {
    let mut sigs2 = sigs.to_vec();
    sigs2.sort_unstable();
    sigs2.dedup();
    return sigs2.len();
}

fn partref_naive(data: &[u8], r: &CReader) -> Vec<u32> {
    let mut ids = init_partition_ids_unsafe(data, r);
    let mut part_count = count_parts(&ids);
    for iter in 0..99999999 {
        // let start_time = SystemTime::now();
        let new_ids = repartition_all_inexact_unsafe64(data, r, &ids);
        let new_part_count = count_parts(&new_ids);
        if new_part_count == new_ids.len() || new_part_count == part_count {
            println!("Number of iterations: {}", iter+1);
            return new_ids.iter().map(|id| *id as u32).collect();
        } else {
            ids = new_ids;
            part_count = new_part_count;
        }
    }
    panic!("Ran out of iterations.")
}

#[test]
fn test_partref_naive() {
    let (data,r) = read_boa_txt("tests/test1.boa.txt");
    let ids = renumber(&partref_naive(&data,&r));
    assert_eq!(&ids, &vec![0,0,1,1,2,3,3,4]);
}


fn renumber<A> (ids: &[A]) -> Vec<u32>
where A:Hash+Eq {
    let mut canon_map = HMap::default();
    let mut last_id = 0;
    let res = ids.iter().map(|id| {
        if canon_map.contains_key(&id) {
            canon_map[&id]
        } else {
            canon_map.insert(id, last_id);
            last_id += 1;
            last_id - 1
        }
    }).collect();
    // println!("Canon map size: {}", data_size(&canon_map));
    return res;
}

fn renumber_sort<A> (sigs: &[A]) -> Vec<u32>
where A:Ord+Copy {
    let mut xs:Vec<(u32,A)> = (0..sigs.len()).map(|i| { (i as u32,sigs[i]) }).collect();
    xs.sort_unstable_by_key(|kv| kv.1);
    let mut ids:Vec<u32> = vec![0;sigs.len()];
    let mut id = 0;
    let mut last_sig = xs[0].1;
    for (i,sig) in xs {
        if sig != last_sig {
            id += 1;
            last_sig = sig;
        }
        ids[i as usize] = id;
    }
    // make sure the first id is 0
    // n log n algorithm relies on this (but could improve it so that it doesn't)
    if ids[0] != 0 {
        for id in ids.iter_mut() {
            if *id == 0 { *id = 1 }
            else if *id == 1 { *id = 0 }
        }
    }
    return ids
}

#[test]
fn test_renumber_sort() {
    assert_eq!(renumber_sort(&vec![3,1,3,1,5,3,0,1]), vec![0,1,0,1,2,0,3,1]);
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

type State = u32;

#[derive(DataSize)]
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

fn partref_nlogn_raw(data: Vec<u8>, r: CReader) -> Vec<ID> {
    // println!("===================== Starting partref_nlogn");
    // panic!("Stopped");
    let coa = Coalg::new(data, r);
    // coa.dump();
    // coa.dump_backrefs();
    let mut iters = 0;
    let mut parts = DirtyPartitions::new(coa.num_states());
    while let Some(partition_id) = parts.worklist.pop() {
        let states = parts.refiners(partition_id);
        // println!("states = {:?}", states);
        let signatures = renumber::<u64>(&repartition_inexact_unsafe(&coa, states, &parts.partition_id));
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
    }
    println!("Number of iterations: {} ", iters);
    println!("DirtyPartitions size: {}, Coalg size: {}", mb(data_size(&parts)), mb(data_size(&coa)));
    println!("Coalg sizes {{ \n  data: {}, \n  reader: {}, \n  locs: {}, \n  backrefs: {}, \n  backrefs_locs: {} \n}}",
        mb(data_size(&coa.data)), mb(data_size(&coa.reader)), mb(&coa.locs.len()*8), mb(data_size(&coa.backrefs)), mb(data_size(&coa.backrefs_locs)));

    // struct Coalg {
    //     data: Vec<u8>, // binary representation of the coalgebra
    //     reader: CReader,
    //     #[data_size(with = ptrvec_datasize)]
    //     locs: Vec<*const u8>, // gives the location in data where the i-th state starts
    //     backrefs: Vec<u32>, // buffer of backrefs
    //     backrefs_locs: Vec<u32> // backrefs_locs[i] gives the index into backrefs[backrefs_locs[i]] where the backrefs of the i-th state start
    // }

    // println!("===================== Ending partref_nlogn");
    return parts.partition_id;
}

fn partref_nlogn(data: Vec<u8>, r: CReader) -> Vec<ID> {
    let ids = partref_nlogn_raw(data, r);
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
    let (data,r) = read_boa_txt("tests/test1.boa.txt");
    let ids1 = partref_naive(&data, &r);
    let ids2 = partref_nlogn(data, r);
    assert_eq!(&renumber(&ids1), &ids2);

    let (data,r) = read_boa_txt("tests/test2.boa.txt");
    let ids = partref_nlogn(data, r);
    assert_eq!(&ids, &vec![0,1,2,3,4,5]);
}

#[test]
fn test_partref_wlan() {
    let filename = "benchmarks/small/wlan0_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_582327_771088_roundrobin_4.boa.txt";
    let (data,r) = read_boa_txt(&filename);
    let ids = partref_nlogn(data, r);
    assert_eq!(*ids.iter().max().unwrap(), 107864);

    let filename = "benchmarks/wlan/wlan1_time_bounded.nm_TRANS_TIME_MAX=10,DEADLINE=100_1408676_1963522_roundrobin_32.boa.txt";
    let (data, r) = read_boa_txt(&filename);
    let ids = partref_nlogn(data, r);
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

use clap::{Parser, ArgEnum};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Action {
    Convert,
    Naive,
    Nlogn,
}

#[derive(Parser,Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(arg_enum)]
    action: Action,

    file: String,
}

fn main() {
    let args = Args::parse();
    match args.action {
        Action::Convert => {
            println!("Converting {}...", &args.file);
            convert_file(&args.file);
            println!("Converted {}.", &args.file);
        },
        Action::Naive|Action::Nlogn => {
            let mut start_time = SystemTime::now();
            println!("Starting parsing {}... ", &args.file);
            let (data,r) = read_boa(&args.file);
            let parsing_time = start_time.elapsed().unwrap();
            println!("Parsing done, size: {} in {} seconds", mb(data.len()), parsing_time.as_secs_f32());
            start_time = SystemTime::now();
            let ids = if args.action == Action::Naive {
                println!("Naive algorithm.");
                partref_naive(&data, &r)
            } else {
                println!("N log N algorithm.");
                partref_nlogn(data, r)
            };
            println!("Number of states: {}, Number of partitions: {}", ids.len(), ids.iter().max().unwrap()+1);
            let computation_time = start_time.elapsed().unwrap();
            println!("Computation took {} seconds", computation_time.as_secs_f32());
        },
    }
}
