use super::{Archive, Unpacked};
use crate::{
    group::Group,
    js5_compression::Js5Compression,
    js5_index::{Js5Index, Js5IndexEntry},
    store::Store,
};
use std::collections::HashMap;

pub struct CacheArchive {
    pub is_dirty: bool,
    pub index: Js5Index,
    pub archive: u8,
    pub unpacked_cache: HashMap<u64, Unpacked>,
}

impl CacheArchive {
    pub fn test_func() {}
}

impl Archive for CacheArchive {
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn read(&self, group: u32, file: u16, key: Option<[u32; 4]>, store: &dyn Store) -> Vec<u8> {
        let entry = self.index.groups.get(&group).unwrap();
        let unpacked = self.get_unpacked(entry, group, key, store);
        unpacked.read(file as u32)
    }

    fn read_named_group(
        &self,
        group_name_hash: u32,
        file: u16,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8> {
        let entry_id = self.index.get_named(group_name_hash).unwrap();
        let entry = self.index.groups.get(&entry_id).unwrap();

        let unpacked = self.get_unpacked(entry, entry_id, key, store);
        unpacked.read(file as u32)
    }

    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Unpacked {
        // TODO: Handle unpacked cache aka cached version of the unpacked files

        let compressed = self.read_packed(entry_id, store);

        self.verify_compressed(&compressed, entry);

        let buf = Js5Compression::uncompress(compressed, key);

        self.verify_uncompressed(&buf, entry);

        let files = Group::unpack(buf, &self.index.groups.get(&entry_id).unwrap().files);

        Unpacked {
            dirty: false,
            key,
            files,
        }
    }

    fn read_packed(&self, group: u32, store: &dyn Store) -> Vec<u8> {
        store.read(self.archive, group)
    }

    // TODO: Implement
    fn verify_compressed(&self, buf: &[u8], entry: &Js5IndexEntry) {}

    // TODO: Implement
    fn verify_uncompressed(&self, buf: &[u8], entry: &Js5IndexEntry) {}
}
