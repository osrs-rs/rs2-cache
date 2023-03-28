use crate::{
    archive::{cache_archive::CacheArchive, Archive},
    djb2::djb2_hash,
    js5_compression::Js5Compression,
    js5_index::Js5Index,
    store::{store_open, Store},
    Cache,
};
use std::{collections::HashMap, io};
use thiserror::Error;

const ARCHIVESET: usize = (1 << 24) - 1;
const UNPACKED_CACHE_SIZE_DEFAULT: usize = 1024;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("data store disconnected")]
    Disconnect(#[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data store error")]
    Unknown,
}

impl Cache {
    pub fn open(input_path: &str) -> io::Result<Cache> {
        Self::open_with_store(store_open(input_path))
    }

    pub fn open_with_store(store: Box<dyn Store>) -> io::Result<Cache> {
        let mut cache = Self {
            store,
            archives: HashMap::new(),
            _unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
        };
        cache.init();

        // Return the Cache struct
        Ok(cache)
    }

    fn init(&mut self) {
        for archive in self.store.list(ARCHIVESET as u8) {
            let compressed = self.store.read(ARCHIVESET as u8, archive);

            let buf = Js5Compression::uncompress(compressed, None);

            let js5_index = Js5Index::read(buf);

            let cache_archive = CacheArchive {
                is_dirty: false,
                index: js5_index,
                archive: archive as u8,
                unpacked_cache: HashMap::new(),
            };

            self.archives.insert(archive as u8, cache_archive);
        }
    }

    /// Read a file from the cache
    ///
    /// # Arguments
    ///
    /// * `archive` - The archive to read from
    /// * `group` - The group to read from
    /// * `file` - The file to read
    /// * `xtea_keys` - The XTEA keys to use for decryption. If None, the file will not be decrypted
    pub fn read(&self, archive: u8, group: u32, file: u16, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        self.archives[&archive].read(group, file, xtea_keys, self.store.as_ref())
    }

    pub fn read_named_group(
        &self,
        archive: u8,
        group: &str,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
    ) -> Vec<u8> {
        self.archives[&archive].read_named_group(
            djb2_hash(group),
            file,
            xtea_keys,
            self.store.as_ref(),
        )
    }
}
