use hmap::HMap;

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

use datasize::DataSize;

use crate::{hmap, parsing};

/// Compression tag
fn is_compressed32(w: u32) -> bool {
    w & 1 == 0
}
fn get_compressed32(w: u32) -> u8 {
    (w as u8) >> 1
}
fn get_noncompressed32(w: u32) -> u32 {
    w >> 1
}
fn put_noncompressed32(w: u32) -> u32 {
    (w << 1) | 1
}

fn is_compressed64(w: u64) -> bool {
    w & 1 == 0
}
fn get_compressed64(w: u64) -> u8 {
    (w as u8) >> 1
}
fn get_noncompressed64(w: u64) -> u64 {
    w >> 1
}
fn put_noncompressed64(w: u64) -> u64 {
    (w << 1) | 1
}

fn put_compressed8(w: u8) -> u8 {
    w << 1
}

/// Assumes that the compressed bit has already been removed
pub fn is_state(w: u32) -> bool {
    w & 1 == 0
}
pub fn get_state(w: u32) -> u32 {
    w >> 1
}
pub fn get_header(w: u32) -> u32 {
    w >> 1
}
pub fn put_state(w: u32) -> u32 {
    w << 1
}
pub fn put_header(w: u32) -> u32 {
    (w << 1) | 1
}

/// Assumes that the header tag has already been removed
pub fn decode_header(w: u32) -> (u8, u8, u16) {
    ((w >> 24) as u8, (w >> 16) as u8, w as u16)
}
pub fn encode_header(typ: u8, tag: u8, len: u16) -> u32 {
    ((typ as u32) << 24) | ((tag as u32) << 16) | (len as u32)
}

pub const LIST_TYP: u8 = 0;
pub const SET_TYP: u8 = 1;
pub const ADD_TYP: u8 = 2;
pub const MAX_TYP: u8 = 3;
pub const OR_TYP: u8 = 4;
pub const TAG_TYP: u8 = 5;

#[test]
fn test_binary_representation() {
    assert_eq!(decode_header(encode_header(1, 2, 3)), (1, 2, 3));
    assert_eq!(
        decode_header(encode_header(1, u8::MAX, u16::MAX)),
        (1, u8::MAX, u16::MAX)
    );
    assert_eq!(
        decode_header(get_header(put_header(encode_header(
            127,
            u8::MAX,
            u16::MAX
        )))),
        (127, u8::MAX, u16::MAX)
    );
    assert_eq!(
        decode_header(get_header(get_noncompressed32(put_noncompressed32(
            put_header(encode_header(63, u8::MAX, u16::MAX))
        )))),
        (63, u8::MAX, u16::MAX)
    );
}

//=========================================//
// Dictionary compressed readers & writers //
//=========================================//

#[derive(DataSize)]
pub struct CReader {
    pub headers: [u32; 128],
    pub values: [u64; 128],
}

impl CReader {
    pub unsafe fn read_node(self: &Self, data: *const u8) -> (u32, *const u8) {
        let x = *(data as *const u32);
        if is_compressed32(x) {
            (self.headers[get_compressed32(x) as usize], data.add(1))
        } else {
            (get_noncompressed32(x), data.add(4))
        }
    }

    pub unsafe fn read_value(self: &Self, data: *const u8) -> (u64, *const u8) {
        let x = *(data as *const u64);
        if is_compressed64(x) {
            (self.values[get_compressed64(x) as usize], data.add(1))
        } else {
            (get_noncompressed64(x), data.add(8))
        }
    }

    pub unsafe fn read_node_mut(self: &Self, data: &mut *const u8) -> u32 {
        let (x, data2) = self.read_node(*data);
        *data = data2;
        return x;
    }

    pub unsafe fn read_value_mut(self: &Self, data: &mut *const u8) -> u64 {
        let (x, data2) = self.read_value(*data);
        *data = data2;
        return x;
    }

    pub unsafe fn is_at_end(data: &[u8], p: *const u8) -> bool {
        return data.as_ptr().add(data.len()) == p;
    }
}

pub struct CWriter {
    pub headers_map: HMap<u32, u8>,
    pub values_map: HMap<u64, u8>,
    pub headers: [u32; 128],
    pub values: [u64; 128],
    pub data: Vec<u8>,
}

impl CWriter {
    pub fn new() -> CWriter {
        CWriter {
            headers_map: HMap::default(),
            values_map: HMap::default(),
            headers: [0; 128],
            values: [0; 128],
            data: vec![],
        }
    }

    pub fn finish(mut self: Self) -> (Vec<u8>, CReader) {
        self.data.reserve(7); // make sure to not trigger undefined behaviour by reading u64 at the last byte
        return (
            self.data,
            CReader {
                headers: self.headers,
                values: self.values,
            },
        );
    }

    pub fn write_node(self: &mut Self, node: u32) {
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
                self.data
                    .extend(u32::to_ne_bytes(put_noncompressed32(node)))
            }
        }
    }

    pub fn write_node_noncompressed(self: &mut Self, node: u32) {
        self.data
            .extend(u32::to_ne_bytes(put_noncompressed32(node)))
    }

    pub fn write_value(self: &mut Self, value: u64) {
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
                self.data
                    .extend(u64::to_ne_bytes(put_noncompressed64(value)))
            }
        }
    }
}

#[test]
fn test_creader_cwriter() {
    let mut w = CWriter::new();
    for _ in 0..10 {
        for i in 0..1000 {
            w.write_node(i)
        }
        for i in 0..1000 {
            w.write_value(i)
        }
    }
    let (data, r) = w.finish();
    assert_eq!(
        data.len(),
        10 * (128 + (1000 - 128) * 4 + 128 + (1000 - 128) * 8)
    );
    let mut p = data.as_ptr();
    unsafe {
        for _ in 0..10 {
            for i in 0..1000 {
                assert_eq!(r.read_node_mut(&mut p), i)
            }
            for i in 0..1000 {
                assert_eq!(r.read_value_mut(&mut p), i)
            }
        }
    }
}

//=================================//
// Convert between text and binary //
//=================================//

#[derive(PartialEq, Debug)]
pub enum Node {
    State(u32),
    Coll(u8, u8, Vec<Node>),
    Mon(u8, u8, Vec<(Node, u64)>),
}

impl Node {
    pub fn from_ascii(inp: &[u8]) -> Self {
        let (node, rest) = parsing::read_node(inp);
        if rest.len() == 0 || rest == [b'\n'] {
            return node;
        } else {
            panic!("Did not parse everything on the line.")
        }
    }

    pub fn to_ascii(self: &Self, w: &mut Vec<u8>) {
        match self {
            Node::State(state) => {
                w.push(b'@');
                w.extend(lexical::to_string(*state).as_bytes());
            }
            Node::Coll(typ, tag, nodes) => {
                let typ_str = match *typ {
                    LIST_TYP => "List[",
                    SET_TYP => "Set[",
                    _ => panic!("Bad typ."),
                };
                w.extend(typ_str.as_bytes());
                w.extend(lexical::to_string(*tag).as_bytes());
                w.extend([b']', b'{']);
                for node in nodes {
                    node.to_ascii(w);
                    w.push(b',');
                }
                if nodes.len() > 0 {
                    w.pop();
                }
                w.push(b'}');
            }
            Node::Mon(typ, tag, nodes) => {
                let typ_str = match *typ {
                    ADD_TYP => "Add[",
                    OR_TYP => "Or[",
                    MAX_TYP => "Max[",
                    TAG_TYP => "Tag[",
                    _ => panic!("Bad typ."),
                };
                w.extend(typ_str.as_bytes());
                w.extend(lexical::to_string(*tag).as_bytes());
                w.extend([b']', b'{']);
                for (node, val) in nodes {
                    node.to_ascii(w);
                    w.push(b':');
                    w.extend(lexical::to_string(*val).as_bytes());
                    w.push(b',');
                }
                if nodes.len() > 0 {
                    w.pop();
                }
                w.push(b'}');
            }
        }
    }

    pub fn write(self: &Self, w: &mut CWriter) {
        match self {
            Node::State(state) => w.write_node_noncompressed(put_state(*state)),
            Node::Coll(typ, tag, nodes) => {
                w.write_node(put_header(encode_header(*typ, *tag, nodes.len() as u16)));
                for node in nodes {
                    node.write(w)
                }
            }
            Node::Mon(typ, tag, nodes) => {
                w.write_node(put_header(encode_header(*typ, *tag, nodes.len() as u16)));
                for (node, val) in nodes {
                    node.write(w);
                    w.write_value(*val)
                }
            }
        }
    }

    pub unsafe fn read(r: &CReader, p: &mut *const u8) -> Self {
        let w = r.read_node_mut(p);
        if is_state(w) {
            Node::State(get_state(w))
        } else {
            let (typ, tag, len) = decode_header(get_header(w));
            match typ {
                LIST_TYP | SET_TYP => {
                    let nodes = (0..len).map(|_| Node::read(r, p)).collect();
                    Node::Coll(typ, tag, nodes)
                }
                ADD_TYP | OR_TYP | MAX_TYP | TAG_TYP => {
                    let nodes = (0..len)
                        .map(|_| {
                            let node = Node::read(r, p);
                            let val = r.read_value_mut(p);
                            (node, val)
                        })
                        .collect();
                    Node::Mon(typ, tag, nodes)
                }
                _ => {
                    panic!("Unknown typ.")
                }
            }
        }
    }
}

#[test]
fn test_node_read_write() {
    // Test conversion from & to ascii
    let node_str =
        "Max[123]{@12:1,Set[123]{@12,@13,@14}:2,Max[123]{@12:3,@13:4,@14:5}:6,Set[12]{}:7}";
    let node = Node::from_ascii(node_str.as_bytes());
    let mut out = vec![];
    node.to_ascii(&mut out);
    assert_eq!(String::from_utf8(out).unwrap(), node_str);

    // Test conversion to & from binary
    let mut w = CWriter::new();
    node.write(&mut w);
    let (data, r) = w.finish();
    unsafe {
        let node2 = Node::read(&r, &mut data.as_ptr());
        assert_eq!(node, node2);
    }
}
