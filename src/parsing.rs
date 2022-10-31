
use crate::{binrep::{Node, LIST_TYP, ADD_TYP, SET_TYP, OR_TYP, MAX_TYP, TAG_TYP}};

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
      b'T' => {
          if inp.len() < 2 || inp[0..2] != [b'a', b'g'] {
              panic!("Expected \"Tag\", got {:?}", String::from_utf8(orig.to_vec()).unwrap());
          }
          return read_mon(&inp[2..], TAG_TYP);
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