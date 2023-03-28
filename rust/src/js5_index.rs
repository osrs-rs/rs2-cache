use osrs_bytes::ReadExt;
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
};

pub enum Js5Protocol {
    Original = 5,
    Versioned = 6,
    Smart = 7,
}

enum Js5IndexFlags {
    Names = 0x1,
    Digests = 0x2,
    Lengths = 0x4,
    UncompressedChecksums = 0x8,
}

#[derive(Debug, PartialEq)]
pub struct Js5IndexFile {
    pub name_hash: i32,
}

#[derive(Debug, PartialEq)]
pub struct Js5IndexEntry {
    pub name_hash: i32,
    pub version: u32,
    pub checksum: u32,
    pub uncompressed_checksum: u32,
    pub length: u32,
    pub uncompressed_length: u32,
    pub digest: Vec<u8>,
    pub capacity: u32,
    pub files: BTreeMap<u32, Js5IndexFile>,
}

#[derive(Debug, PartialEq)]
pub struct Js5Index {
    pub protocol: u8,
    pub version: i32,
    pub has_names: bool,
    pub has_digests: bool,
    pub has_lengths: bool,
    pub has_uncompressed_checksums: bool,
    pub groups: BTreeMap<u32, Js5IndexEntry>,
    pub name_hash_table: HashMap<u32, u32>,
}

impl Js5Index {
    pub fn read<T: AsRef<[u8]>>(buf: T) -> Js5Index {
        let mut buf_ref = buf.as_ref();

        let protocol = buf_ref.read_u8().unwrap();

        let read_func = if protocol >= Js5Protocol::Smart as u8 {
            |v: &mut &[u8]| -> u32 { v.read_u32_smart().unwrap() }
        } else {
            |v: &mut &[u8]| -> u32 { v.read_u16().unwrap() as u32 }
        };

        let version = if protocol >= Js5Protocol::Versioned as u8 {
            buf_ref.read_i32().unwrap()
        } else {
            0
        };
        let flags = buf_ref.read_u8().unwrap();
        let size = read_func(&mut buf_ref);

        // Create Js5Index
        let mut index = Js5Index {
            protocol,
            version,
            has_names: (flags & Js5IndexFlags::Names as u8) != 0,
            has_digests: (flags & Js5IndexFlags::Digests as u8) != 0,
            has_lengths: (flags & Js5IndexFlags::Lengths as u8) != 0,
            has_uncompressed_checksums: (flags & Js5IndexFlags::UncompressedChecksums as u8) != 0,
            groups: BTreeMap::new(),
            name_hash_table: HashMap::new(),
        };

        // Begin creating the groups
        let mut prev_group_id = 0;
        (0..size).for_each(|_| {
            prev_group_id += read_func(&mut buf_ref);
            index.groups.insert(
                prev_group_id,
                Js5IndexEntry {
                    name_hash: -1,
                    version: 0,
                    checksum: 0,
                    uncompressed_checksum: 0,
                    length: 0,
                    uncompressed_length: 0,
                    digest: Vec::new(),
                    capacity: 0,
                    files: BTreeMap::new(),
                },
            );
        });

        if index.has_names {
            for (id, group) in index.groups.iter_mut() {
                group.name_hash = buf_ref.read_i32().unwrap();
                index.name_hash_table.insert(group.name_hash as u32, *id);
            }
        }

        for group in index.groups.values_mut() {
            group.checksum = buf_ref.read_u32().unwrap();
        }

        if index.has_uncompressed_checksums {
            for group in index.groups.values_mut() {
                group.uncompressed_checksum = buf_ref.read_u32().unwrap();
            }
        }

        if index.has_digests {
            for group in index.groups.values_mut() {
                let digest_bits = 512;
                let digest_bytes = digest_bits >> 3;
                let mut digest = vec![0; digest_bytes];
                buf_ref.read_exact(&mut digest).unwrap();
                group.digest.extend(&digest);
            }
        }

        if index.has_lengths {
            for group in index.groups.values_mut() {
                group.length = buf_ref.read_u32().unwrap();
                group.uncompressed_length = buf_ref.read_u32().unwrap();
            }
        }

        for group in index.groups.values_mut() {
            group.version = buf_ref.read_u32().unwrap();
        }

        let group_sizes: Vec<u32> = (0..size).map(|_| read_func(&mut buf_ref)).collect();

        for (i, group) in index.groups.values_mut().enumerate() {
            let group_size = group_sizes[i];

            let mut prev_file_id = 0;
            (0..group_size).for_each(|_| {
                prev_file_id += read_func(&mut buf_ref);
                group
                    .files
                    .insert(prev_file_id, Js5IndexFile { name_hash: -1 });
            });
        }

        if index.has_names {
            for group in index.groups.values_mut() {
                for file in group.files.values_mut() {
                    file.name_hash = buf_ref.read_i32().unwrap();
                }
            }
        }

        index
    }

    pub fn get_named(&self, name_hash: u32) -> Option<u32> {
        self.name_hash_table.get(&name_hash).copied()
    }
}
