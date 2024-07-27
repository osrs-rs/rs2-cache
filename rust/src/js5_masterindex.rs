use crate::{
    js5_compression::Js5Compression,
    js5_index::{Js5Index, Js5Protocol},
    store::{Store, ARCHIVESET},
};
use crc32fast::hash;
use osrs_bytes::WriteExt;
use std::{cmp, io::Write};

const MASTERINDEXFORMAT_ORIGINAL: u8 = 0;
const MASTERINDEXFORMAT_VERSIONED: u8 = 1;
const MASTERINDEXFORMAT_DIGESTS: u8 = 2;
const MASTERINDEXFORMAT_LENGTHS: u8 = 3;

pub struct Js5MasterIndexEntry {
    pub version: i32,
    pub checksum: u32,
    pub groups: usize,
    pub total_uncompressed_length: u32,
    pub digest: Option<[u8; 32]>,
}

pub struct Js5MasterIndex {
    pub format: u8,
    pub entries: Vec<Js5MasterIndexEntry>,
}

impl Js5MasterIndex {
    pub fn write(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        if self.format >= MASTERINDEXFORMAT_DIGESTS {
            buf.write_u8(self.entries.len() as u8).unwrap();
        }

        for entry in &self.entries {
            buf.write_u32(entry.checksum).unwrap();

            if self.format >= MASTERINDEXFORMAT_VERSIONED {
                buf.write_i32(entry.version).unwrap();
            }

            if self.format >= MASTERINDEXFORMAT_LENGTHS {
                buf.write_i32(entry.groups as i32).unwrap();
                buf.write_u32(entry.total_uncompressed_length).unwrap();
            }

            if self.format >= MASTERINDEXFORMAT_DIGESTS {
                if let Some(digest) = &entry.digest {
                    buf.write_all(digest).unwrap();
                } else {
                    buf.write_all(&[0; 32]).unwrap();
                }
            }
        }

        // TODO: More digest stuff on masterindex, likely caught by tests, so impl this later

        buf
    }

    pub fn create(store: &Box<dyn Store + Send + Sync>) -> Js5MasterIndex {
        let mut master_index = Js5MasterIndex {
            format: MASTERINDEXFORMAT_ORIGINAL,
            entries: Vec::new(),
        };

        let mut next_archive = 0;
        for archive in store.list(ARCHIVESET).unwrap() {
            let read = store.read(ARCHIVESET, archive).unwrap();

            let checksum = hash(&read);

            let uncompress = Js5Compression::uncompress(read, None).unwrap();

            let index = Js5Index::read(uncompress).unwrap();

            if index.has_lengths {
                master_index.format = cmp::max(master_index.format, MASTERINDEXFORMAT_LENGTHS);
            } else if index.has_digests {
                master_index.format = cmp::max(master_index.format, MASTERINDEXFORMAT_DIGESTS);
            } else if index.protocol >= Js5Protocol::Versioned as u8 {
                master_index.format = cmp::max(master_index.format, MASTERINDEXFORMAT_VERSIONED);
            }

            let version = index.version;
            let groups = index.groups.len();
            let total_uncompressed_length: u32 = index
                .groups
                .iter()
                .map(|group| group.1.uncompressed_length)
                .sum();

            for _ in next_archive..archive {
                master_index.entries.push(Js5MasterIndexEntry {
                    version: 0,
                    checksum: 0,
                    groups: 0,
                    total_uncompressed_length: 0,
                    digest: None,
                });
            }

            master_index.entries.push(Js5MasterIndexEntry {
                version,
                checksum,
                groups,
                total_uncompressed_length,
                digest: None,
            });

            next_archive = archive + 1;
        }

        master_index
    }
}

/*
Implement tests once FlatFileStore is implemented

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::store_open;

    #[test]
    fn test_create_original() {
        let storee = store_open("tests/data/master-index/original").unwrap();

        let index = Js5MasterIndex::create(storee);
    }

    //const INVALID_KEY: [u32; 4] = [0x01234567, 0x89ABCDEF, 0x01234567, 0x89ABCDEF];
}
*/
