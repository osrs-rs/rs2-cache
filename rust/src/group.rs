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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack_single() {
        let actual = Group::unpack(
            vec![0, 1, 2, 3],
            &BTreeMap::from([(1, Js5IndexFile { name_hash: 0 })]),
        );
        let expected = BTreeMap::from([(1, vec![0, 1, 2, 3])]);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_unpack_zero_stripes() {
        let expected = BTreeMap::from([(0, Vec::new()), (1, Vec::new()), (3, Vec::new())]);
        let actual = Group::unpack(
            vec![0],
            &BTreeMap::from([
                (0, Js5IndexFile { name_hash: 0 }),
                (1, Js5IndexFile { name_hash: 0 }),
                (3, Js5IndexFile { name_hash: 0 }),
            ]),
        );

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_unpack_one_stripe() {
        let expected = BTreeMap::from([
            (0, vec![0, 1, 2]),
            (1, vec![3, 4, 5, 6, 7]),
            (3, vec![8, 9]),
        ]);
        let actual = Group::unpack(
            vec![
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0, 3, 0, 0, 0, 2, 0xFF, 0xFF, 0xFF, 0xFD, 1,
            ],
            &BTreeMap::from([
                (0, Js5IndexFile { name_hash: 0 }),
                (1, Js5IndexFile { name_hash: 0 }),
                (3, Js5IndexFile { name_hash: 0 }),
            ]),
        );

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_unpack_multiple_stripe() {
        let expected = BTreeMap::from([
            (0, vec![0, 1, 2]),
            (1, vec![3, 4, 5, 6, 7]),
            (3, vec![8, 9]),
        ]);
        let actual = Group::unpack(
            vec![
                0, 1, 3, 4, 8, 9, 2, 5, 6, 7, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
                0, 2, 0xFF, 0xFF, 0xFF, 0xFD, 2,
            ],
            &BTreeMap::from([
                (0, Js5IndexFile { name_hash: 0 }),
                (1, Js5IndexFile { name_hash: 0 }),
                (3, Js5IndexFile { name_hash: 0 }),
            ]),
        );

        assert_eq!(expected, actual);
    }
}
