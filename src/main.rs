#![allow(dead_code)]
use std::{hash::{Hash}, time::SystemTime};
use hmap::HMap;

mod hmap;
mod util;
mod parsing;
mod io;
mod binrep;
mod coalg;
mod refpart;
mod naivealg;
mod optalg;


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
    // TODO: Try sorting array 0..n by key sigs[i]
    let mut xs:Vec<u32> = (0..sigs.len() as u32).collect();
    xs.sort_unstable_by_key(|i| sigs[*i as usize]);
    let mut ids:Vec<u32> = vec![0;sigs.len()];
    let mut id = 0;
    let mut last_sig = sigs[xs[0] as usize];
    for i in xs {
        let sig = sigs[i as usize];
        if sig != last_sig {
            id += 1;
            last_sig = sig;
        }
        ids[i as usize] = id;
    }
    // make sure the first id is 0
    // n log n algorithm relies on this (but could improve it so that it doesn't)
    let firstid = ids[0];
    if firstid != 0 {
        for id in ids.iter_mut() {
            if *id == 0 { *id = firstid }
            else if *id == firstid { *id = 0 }
        }
    }
    return ids
}

#[test]
fn test_renumber_sort() {
    assert_eq!(renumber_sort(&vec![3,1,3,1,5,3,0,1]), vec![0,1,0,1,3,0,2
    ,1]);
}





use clap::{Parser, ArgEnum};

use crate::{io::{convert_file, read_boa}, naivealg::partref_naive, optalg::partref_nlogn};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Action {
    Convert,
    Naive,
    Nlogn,
}

/// Binary coalgebraic partition refinement.\n\

#[derive(Parser,Debug)]
#[clap(author, version,
about = "Binary coalgebraic partition refinement.\n\
- Use `boa convert file.boa.txt` to convert a text file to binary format.\n\
- Use `boa convert file.boa` to convert a binary file to text format.\n\
- Use `boa naive file.boa` to run the naive algorithm.\n\
- Use `boa nlogn file.boa` to run the nlogn algorithm.", long_about = None)]
struct Args {
    #[clap(arg_enum)]
    action: Action,

    file: String,
}

fn main() {
    let args = Args::parse();
    match args.action {
        Action::Convert => {
            println!("file: {}", &args.file);
            convert_file(&args.file);
        },
        Action::Naive|Action::Nlogn => {
            let mut start_time = SystemTime::now();
            println!("file: {}", &args.file);
            let (data,r) = read_boa(&args.file);
            let parsing_time = start_time.elapsed().unwrap();
            println!("size_mb: {}", util::mb(data.len()));
            println!("parsing_time_s: {}", parsing_time.as_secs_f32());
            start_time = SystemTime::now();
            let ids = if args.action == Action::Naive {
                println!("algorithm: naive");
                partref_naive(&data, &r)
            } else {
                println!("algorithm: nlogn");
                partref_nlogn(data, r)
            };
            let computation_time = start_time.elapsed().unwrap();
            println!("n_states: {}", ids.len());
            println!("n_states_min: {}", ids.iter().max().unwrap()+1);
            println!("reduction_time_s: {}", computation_time.as_secs_f32());
        },
    }
}
