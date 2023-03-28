use crate::js5_index::Js5IndexFile;
use osrs_bytes::ReadExt;
use std::{collections::BTreeMap, io::Cursor};

pub struct Group {}

impl Group {
    pub fn unpack(input: Vec<u8>, group: &BTreeMap<u32, Js5IndexFile>) -> BTreeMap<u32, Vec<u8>> {
        if group.is_empty() {
            panic!("Group has no files")
        }

        if group.len() == 1 {
            let single_entry = group.keys().next().unwrap();
            let mut files = BTreeMap::new();
            files.insert(*single_entry, input);
            return files;
        }

        let mut input_reader = Cursor::new(&input);

        // Now begin going over the stripes
        let stripes = *input.last().unwrap();

        let mut data_index = input_reader.position() as i32;
        let trailer_index = input.len() - (stripes as usize * group.len() * 4) - 1;

        input_reader.set_position(trailer_index as u64);

        let mut lens = vec![0; group.len()];
        for _ in 0..stripes {
            let mut prev_len = 0;
            for j in lens.iter_mut() {
                prev_len += input_reader.read_i32().unwrap();
                *j += prev_len;
            }
        }

        input_reader.set_position(trailer_index as u64);

        let mut files = BTreeMap::new();
        for (i, x) in group.keys().enumerate() {
            files.insert(*x, Vec::with_capacity(lens[i] as usize));
        }

        for _ in 0..stripes {
            let mut prev_len = 0;
            for x in group.keys() {
                prev_len += input_reader.read_i32().unwrap();
                let dst = files.get_mut(x).unwrap();
                let cap = dst.capacity();
                dst.extend_from_slice(
                    &input[data_index as usize..(data_index + prev_len) as usize],
                );
                // Truncate to the correct length in case the buffer has
                // too much data pushed into it.
                // In OpenRS2 it a hard limit which is not supported in Rust
                dst.truncate(cap);

                data_index += prev_len;
            }
        }

        files
    }
}
