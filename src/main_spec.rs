use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::HashMap;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn state_map(filename: &str) -> HashMap<String,u32> {
  let mut states : HashMap<String,u32> = HashMap::new();
  let mut last_state : u32 = 0;

  if let Ok(lines) = read_lines(filename) {
    for line in lines {
      if let Ok(l) = line {
        if let Some(i) = l.find(":") {
          if l.chars().next().unwrap() != '#' {
            let state_name = &l[0..i];
            if !states.contains_key(state_name) {
              states.insert(String::from(state_name), last_state);
              last_state += 1;
            } else {
              panic!("Duplicate state: {}.", state_name);
            }
          }
        }
      } else {
        panic!("Failed to parse line.")
      }
    }
  } else {
    panic!("Couldn't open file {}.", filename);
  }
  return states
}

fn main() {
  let filename = "benchmarks/small/fms.sm_n=4_35910_237120_roundrobin_4.coalgebra";
  // let filename = "benchmarks/large/wta_powerset_0,0,0,4_65000000_2_1300000_32.coalgebra";
  // let filename = "benchmarks/fms/fms.sm_n=8_4459455_38533968_roundrobin_32.coalgebra";
  let sm = state_map(filename);
  println!("{}", sm.len());
  let inp = "0s0   ";
  let out = State::parse(sm, inp);
  println!("out: {:?}", out);
}

// Write converter for COPAR -> binary format
// Write algorithm for binary format
// Start with R^X

struct Inp {
  buf : Vec<char>,
  pos : usize
}

trait Functor {
  /// Parses a line of text into the binary representation
  fn parse(states: HashMap<String, u32>, inp : &mut Inp) -> Vec<u8>;
}

struct State {}

impl Functor for State {
  fn parse(states : HashMap<String,u32>, inp : &mut Inp) -> Vec<u8> {
    map_res(terminated(alphanumeric1,many0(char(' '))), |x| {
      if states.contains_key(x) {
        Ok(Vec::from(states[x].to_ne_bytes()))
      } else {
        Err(())
      }
    })(inp)
  }
}

use std::marker::PhantomData;
struct R<A> {
  phantom: PhantomData<A>
}

impl<X> Functor for R<X> where X : Functor {
  fn parse(states : HashMap<String,u32>, inp : &str) -> IResult<&str,Vec<u8>> {
    let r = delimited(
      terminated(char('{'),many0(char(' '))),
      separated_list0(terminated(char(','), many0(char(' '))),
        X::parse
      ),
      terminated(char('}'),many0(char(' '))))(inp)
  }
}

fn foo(states : &HashMap<String,u32>) -> impl Fn(&str) -> IResult<&str,Vec<u8>> {
  map_res(terminated(alphanumeric1,many0(char(' '))), |x:&str| {
    if states.contains_key(x) {
      Ok(Vec::from(states[x].to_ne_bytes()))
    } else {
      Err(())
    }
  })
}


// impl<A> Functor for R<A> where A : Functor {
//   fn parse(inp : &str) {
//     println!("Tuple start");
//     A::parse(inp);
//     println!("Tuple end");
//   }
// }

// fn main() {
//   R::<State>::parse("Blah");
//   let s = String::from("Hello");
//   let bs = s.as_bytes();
//   if bs[0] == ('a' as u8) {
//     println!("Yes");
//   } else {
//     println!("No");
//   }
// }