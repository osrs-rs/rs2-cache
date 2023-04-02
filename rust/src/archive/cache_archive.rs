use super::{Archive, ArchiveError, Unpacked};
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
    pub unpacked_cache: HashMap<u32, Unpacked>,
}

impl CacheArchive {
    pub fn _test_func() {}
}

impl Archive for CacheArchive {
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn read(
        &mut self,
        group: u32,
        file: u16,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<Vec<u8>, ArchiveError> {
        let unpacked = match self.unpacked_cache.get(&group) {
            Some(unpacked) => unpacked,
            None => self.get_unpacked(group, key, store)?,
        };
        Ok(unpacked.read(file as u32)?)
    }

    fn read_named_group(
        &mut self,
        group_name_hash: u32,
        file: u16,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<Vec<u8>, ArchiveError> {
        let entry_id = self.index.get_named(group_name_hash)?;
        let unpacked = match self.unpacked_cache.get(&entry_id) {
            Some(unpacked) => unpacked,
            None => self.get_unpacked(entry_id, key, store)?,
        };
        Ok(unpacked.read(file as u32)?)
    }

    fn get_unpacked(
        &mut self,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<&Unpacked, ArchiveError> {
        let entry = self
            .index
            .groups
            .get(&entry_id)
            .ok_or(ArchiveError::GroupNotFound(entry_id))?;

        let compressed = self.read_packed(entry_id, store)?;

        self.verify_compressed(&compressed, entry);

        let buf = Js5Compression::uncompress(compressed, key)?;

        self.verify_uncompressed(&buf, entry);

        let files = Group::unpack(
            buf,
            &self
                .index
                .groups
                .get(&entry_id)
                .ok_or(ArchiveError::GroupNotFound(entry_id))?
                .files,
        )?;

        Ok(self.unpacked_cache.entry(entry_id).or_insert(Unpacked {
            _dirty: false,
            _key: key,
            files,
        }))
    }

    fn read_packed(&self, group: u32, store: &dyn Store) -> Result<Vec<u8>, ArchiveError> {
        Ok(store.read(self.archive, group)?)
    }

    // TODO: Implement
    fn verify_compressed(&self, _buf: &[u8], _entry: &Js5IndexEntry) {}

    // TODO: Implement
    fn verify_uncompressed(&self, _buf: &[u8], _entry: &Js5IndexEntry) {}
}
