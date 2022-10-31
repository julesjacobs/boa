use std::io::{BufRead, BufWriter, Write, Read};
use std::{path::Path, fs::File, io::BufReader};

use byteorder::{ReadBytesExt, LittleEndian, WriteBytesExt};

use crate::binrep::{CReader, TAG_TYP};
use crate::binrep::CWriter;
use crate::binrep::Node;
use crate::hmap::HMap;

pub fn read_boa_txt<P>(filename: P) -> (Vec<u8>,CReader)
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

pub fn create_file<P>(filename: P) -> File
where P: AsRef<Path>, {
    if filename.as_ref().exists() { println!("File already exists: {:?}", filename.as_ref().display().to_string()) }
    let file = File::create(&filename).
        expect(&format!("Couldn't create file {:?}", filename.as_ref().display().to_string()));
    return file
}

pub fn write_boa_txt<P>(filename: P, data: &[u8], r: &CReader)
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
            writer.write_all(&buf).unwrap();
            buf.clear();
        }
    }
}

pub fn read_boa<P>(filename: P) -> (Vec<u8>,CReader)
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

pub fn write_boa<P>(filename: P, data: &[u8], r: &CReader)
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
        writer.write_u64::<LittleEndian>(value).expect("Writing error.");
    }
    writer.write_all(data).expect("Writing error.");
}

pub fn read_aut<P>(filename: P) -> (Vec<u8>,CReader)
where P: AsRef<Path>, {
    let filename_str = filename.as_ref().display().to_string();
    if !filename_str.ends_with(".aut") {
        panic!("File must be *.aut, but is {}", filename_str);
    }
    let file = File::open(&filename).
        expect(&format!("Couldn't open file {:?}", filename.as_ref().display().to_string()));
    let mut reader = BufReader::new(file);
    let mut line = vec![];
    let _n = reader.read_until(b'\n', &mut line).expect("Failure while reading file.");
    debug_assert!(&line[0..8] == [b'd', b'e', b's', b' ', b'(', b'0', b',', b' ']);
    let line = &line[8..];
    let (_num_edges,m) = lexical::parse_partial::<u32,_>(&line).expect("Expected a number in aut header (1).");
    let line = &line[m..];
    debug_assert!(&line[0..2] == [b',', b' ']);
    let line = &line[2..];
    let (num_states,_m2) = lexical::parse_partial::<u32,_>(&line).expect("Expected a number in aut header (2).");

    let mut states : Vec<Vec<(u64,u32)>> = vec![];
    for _ in 0..num_states { states.push(vec![]); }

    let mut label_counter = 0;
    let mut label_map : HMap<Vec<u8>,u64> = HMap::default();

    let mut line = vec![];
    while 0 < reader.read_until(b'\n', &mut line).expect("Failure while reading file.") {
        let (source,m1) = lexical::parse_partial::<u32,_>(&line[1..]).expect("Expected a number in aut body.");
        line.reverse();
        let (_target,m2) = lexical::parse_partial::<u32,_>(&line[2..]).expect("Expected a number in aut body.");
        line.reverse();
        let label_start = 1+m1;
        let label_end = line.len() - 2 - m2;
        let (target,_m2) = lexical::parse_partial::<u32,_>(&line[label_end..]).expect("Expected a number in aut body.");
        let label_str = Vec::from(&line[label_start..label_end]);
        // println!("label: {}", &String::from_utf8(label_str.clone()).expect("UTF8 ERROR"));
        // Parse label
        let label = if label_map.contains_key(&label_str) {
                label_map[&label_str]
            }else{
                label_map.insert(label_str, label_counter);
                label_counter += 1;
                label_counter-1
            };

        states[source as usize].push((label,target));
        line.clear();
    }

    let mut w = CWriter::new();

    for state in states {
        let trans : Vec<(Node,u64)> = state.iter().map(|(a,b)| { (Node::State(*b), *a) }).collect();
        let node = Node::Mon(TAG_TYP, 0, trans);
        node.write(&mut w);
    }

    w.finish()
}

pub fn convert_file(filename: &str) {
    if filename.ends_with(".boa") {
        let new_filename = [&filename[0..filename.len()-4],".boa.txt"].concat();
        let (data,r) = read_boa(filename);
        write_boa_txt(new_filename, &data, &r);
    } else if filename.ends_with(".boa.txt") {
        let new_filename = [&filename[0..filename.len()-8],".boa"].concat();
        let (data,r) = read_boa_txt(filename);
        write_boa(new_filename, &data, &r);
    } else if filename.ends_with(".aut") {
        let new_filename = [&filename[0..filename.len()-4],".boa"].concat();
        let (data,r) = read_aut(filename);
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