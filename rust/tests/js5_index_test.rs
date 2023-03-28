use memmap2::Mmap;
use rs2cache::{djb2_hash, Js5Index, Js5IndexEntry, Js5IndexFile, Js5Protocol};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    path::Path,
};

#[test]
fn test_read_empty() {
    read("empty.dat", |data| {
        let index = Js5Index::read(data);

        let empty_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: false,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups: BTreeMap::new(),
            name_hash_table: HashMap::new(),
        };

        assert_eq!(empty_index, index);
    });
}

#[test]
fn test_read_versioned() {
    read("versioned.dat", |data| {
        let index = Js5Index::read(data);

        let versioned_index = Js5Index {
            protocol: Js5Protocol::Versioned as u8,
            version: 0x12345678,
            has_names: false,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups: BTreeMap::new(),
            name_hash_table: HashMap::new(),
        };

        assert_eq!(versioned_index, index);
    });
}

#[test]
fn test_read_no_flags() {
    read("no-flags.dat", |data| {
        let index = Js5Index::read(data);

        let files_1 = {
            let mut files = BTreeMap::new();
            files.insert(0, Js5IndexFile { name_hash: -1 });
            files
        };
        let files_2 = {
            let mut files = BTreeMap::new();
            files.insert(1, Js5IndexFile { name_hash: -1 });
            files.insert(3, Js5IndexFile { name_hash: -1 });
            files
        };
        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: -1,
                version: 0,
                checksum: 0x01234567,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files: files_1,
            },
        );
        groups.insert(
            1,
            Js5IndexEntry {
                name_hash: -1,
                version: 10,
                checksum: 0x89ABCDEF,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files: BTreeMap::new(),
            },
        );
        groups.insert(
            3,
            Js5IndexEntry {
                name_hash: -1,
                version: 20,
                checksum: 0xAAAA5555,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files: files_2,
            },
        );

        let no_flags_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: false,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups,
            name_hash_table: HashMap::new(),
        };

        assert_eq!(no_flags_index, index);
    });
}

#[test]
fn test_read_named() {
    read("named.dat", |data| {
        let index = Js5Index::read(data);

        let files = {
            let mut files = BTreeMap::new();
            files.insert(
                0,
                Js5IndexFile {
                    name_hash: djb2_hash("world") as i32,
                },
            );
            files
        };
        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: djb2_hash("hello") as i32,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files,
            },
        );
        let name_hash_table = {
            let mut table = HashMap::new();
            table.insert(djb2_hash("hello"), 0);
            table
        };

        let named_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: true,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups: groups,
            name_hash_table,
        };

        assert_eq!(named_index, index);
    });
}

#[test]
fn test_read_smart() {
    read("smart.dat", |data| {
        let index = Js5Index::read(data);

        let files = {
            let mut files = BTreeMap::new();
            files.insert(0, Js5IndexFile { name_hash: -1 });
            files.insert(100000, Js5IndexFile { name_hash: -1 });
            files
        };

        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: -1,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files,
            },
        );
        groups.insert(
            100000,
            Js5IndexEntry {
                name_hash: -1,
                version: 0x5555AAAA,
                checksum: 0xAAAA5555,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files: BTreeMap::new(),
            },
        );

        let smart_index = Js5Index {
            protocol: Js5Protocol::Smart as u8,
            version: 0,
            has_names: false,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups,
            name_hash_table: HashMap::new(),
        };

        assert_eq!(smart_index, index);
    });
}

#[test]
fn test_read_digest() {
    read("digest.dat", |data| {
        let index = Js5Index::read(data);

        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: -1,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0,
                length: 0,
                uncompressed_length: 0,
                digest: vec![
                    25, 250, 97, 215, 85, 34, 164, 102, 155, 68, 227, 156, 29, 46, 23, 38, 197, 48,
                    35, 33, 48, 212, 7, 248, 154, 254, 224, 150, 73, 151, 247, 167, 62, 131, 190,
                    105, 139, 40, 143, 235, 207, 136, 227, 224, 60, 79, 7, 87, 234, 137, 100, 229,
                    155, 99, 217, 55, 8, 177, 56, 204, 66, 166, 110, 179,
                ],
                capacity: 0,
                files: BTreeMap::new(),
            },
        );
        let digest_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: false,
            has_digests: true,
            has_lengths: false,
            has_uncompressed_checksums: false,
            groups,
            name_hash_table: HashMap::new(),
        };

        assert_eq!(digest_index, index);
    });
}

#[test]
fn test_read_lengths() {
    read("lengths.dat", |data| {
        let index = Js5Index::read(data);

        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: -1,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0,
                length: 1000,
                uncompressed_length: 2000,
                digest: Vec::new(),
                capacity: 0,
                files: BTreeMap::new(),
            },
        );
        let lengths_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: false,
            has_digests: false,
            has_lengths: true,
            has_uncompressed_checksums: false,
            groups,
            name_hash_table: HashMap::new(),
        };

        assert_eq!(lengths_index, index);
    });
}

#[test]
fn test_read_uncompressed_checksum() {
    read("uncompressed-checksum.dat", |data| {
        let index = Js5Index::read(data);

        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: -1,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0xAAAA5555,
                length: 0,
                uncompressed_length: 0,
                digest: Vec::new(),
                capacity: 0,
                files: BTreeMap::new(),
            },
        );
        let uncompressed_checksum_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: false,
            has_digests: false,
            has_lengths: false,
            has_uncompressed_checksums: true,
            groups,
            name_hash_table: HashMap::new(),
        };

        assert_eq!(uncompressed_checksum_index, index);
    });
}

#[test]
fn test_read_all_flaags() {
    read("all-flags.dat", |data| {
        let index = Js5Index::read(data);

        let files = {
            let mut files = BTreeMap::new();
            files.insert(
                0,
                Js5IndexFile {
                    name_hash: djb2_hash("world") as i32,
                },
            );
            files
        };

        let name_hash_table = {
            let mut table = HashMap::new();
            table.insert(djb2_hash("hello"), 0);
            table
        };

        let mut groups = BTreeMap::new();
        groups.insert(
            0,
            Js5IndexEntry {
                name_hash: djb2_hash("hello") as i32,
                version: 0x89ABCDEF,
                checksum: 0x01234567,
                uncompressed_checksum: 0xAAAA5555,
                length: 1000,
                uncompressed_length: 2000,
                digest: vec![
                    25, 250, 97, 215, 85, 34, 164, 102, 155, 68, 227, 156, 29, 46, 23, 38, 197, 48,
                    35, 33, 48, 212, 7, 248, 154, 254, 224, 150, 73, 151, 247, 167, 62, 131, 190,
                    105, 139, 40, 143, 235, 207, 136, 227, 224, 60, 79, 7, 87, 234, 137, 100, 229,
                    155, 99, 217, 55, 8, 177, 56, 204, 66, 166, 110, 179,
                ],
                capacity: 0,
                files,
            },
        );
        let all_flags_index = Js5Index {
            protocol: Js5Protocol::Original as u8,
            version: 0,
            has_names: true,
            has_digests: true,
            has_lengths: true,
            has_uncompressed_checksums: true,
            groups,
            name_hash_table,
        };

        assert_eq!(all_flags_index, index);
    });
}

fn read<P, F>(p: P, f: F)
where
    P: AsRef<Path>,
    F: FnOnce(Mmap),
{
    f(unsafe { Mmap::map(&File::open(Path::new("tests/data/index").join(p)).unwrap()) }.unwrap())
}
