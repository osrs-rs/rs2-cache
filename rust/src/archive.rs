use crate::{js5_index::Js5IndexEntry, store::Store};
use std::collections::BTreeMap;

pub mod cache_archive;

pub trait Archive {
    fn is_dirty(&self) -> bool;
    fn read(
        &self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8>;
    fn read_named_group(
        &self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8>;
    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Unpacked;
    fn read_packed(&self, group: u32, store: &dyn Store) -> Vec<u8>;
    fn verify_compressed(&self, buf: &[u8], entry: &Js5IndexEntry);
    fn verify_uncompressed(&self, buf: &[u8], entry: &Js5IndexEntry);
}

pub struct Unpacked {
    _dirty: bool,
    _key: Option<[u32; 4]>,
    files: BTreeMap<u32, Vec<u8>>,
}

impl Unpacked {
    pub fn read(&self, file: u32) -> Vec<u8> {
        self.files.get(&file).unwrap().to_vec()
    }
}
