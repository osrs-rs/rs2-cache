use crate::{
    archive::{cache_archive::CacheArchive, Archive, ArchiveError},
    djb2::djb2_hash,
    js5_compression::{Js5Compression, Js5CompressionError},
    js5_index::{Js5Index, Js5IndexError},
    store::{store_open, Store, StoreError},
    Cache,
};
use std::{collections::HashMap, io};
use thiserror::Error;

const ARCHIVESET: usize = (1 << 24) - 1;
const UNPACKED_CACHE_SIZE_DEFAULT: usize = 1024;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JS5 compression error: {0}")]
    Js5Compression(#[from] Js5CompressionError),
    #[error("JS5 index error: {0}")]
    Js5Index(#[from] Js5IndexError),
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("ArchiveError: {0}")]
    ArchiveError(#[from] ArchiveError),
    #[error("failed getting CacheArchive {0} from cache")]
    ArchiveNotFound(u8),
    #[error("failed reading CacheArchive {0} from cache")]
    ArchiveRead(u8),
}

impl Cache {
    /// Open a cache from a path
    ///
    /// # Arguments
    ///
    /// * `input_path` - The path to the cache
    pub fn open(input_path: &str) -> Result<Cache, CacheError> {
        Self::open_with_store(store_open(input_path)?)
    }

    /// Open a cache from a store
    ///
    /// # Arguments
    ///
    /// * `store` - The store to use
    pub fn open_with_store(store: Box<dyn Store + Send>) -> Result<Cache, CacheError> {
        let mut cache = Self {
            store,
            archives: HashMap::new(),
            _unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
        };
        cache.init()?;

        // Return the Cache struct
        Ok(cache)
    }

    fn init(&mut self) -> Result<(), CacheError> {
        for archive in self.store.list(ARCHIVESET as u8)? {
            let compressed = self.store.read(ARCHIVESET as u8, archive)?;

            let buf = Js5Compression::uncompress(compressed, None)?;

            let js5_index = Js5Index::read(buf)?;

            let cache_archive = CacheArchive {
                is_dirty: false,
                index: js5_index,
                archive: archive as u8,
                unpacked_cache: HashMap::new(),
            };

            self.archives.insert(archive as u8, cache_archive);
        }

        Ok(())
    }

    /// Read a file from the cache
    ///
    /// # Arguments
    ///
    /// * `archive` - The archive to read from
    /// * `group` - The group to read from
    /// * `file` - The file to read
    /// * `xtea_keys` - The XTEA keys to use for decryption. If None, the file will not be decrypted
    pub fn read(
        &mut self,
        archive: u8,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
    ) -> Result<Vec<u8>, CacheError> {
        Ok(self
            .archives
            .get_mut(&archive)
            .ok_or(CacheError::ArchiveNotFound(archive))?
            .read(group, file, xtea_keys, self.store.as_ref())?)
    }

    /// Read a file from the cache using a named group
    ///
    /// # Arguments
    ///
    /// * `archive` - The archive to read from
    /// * `group` - The group to read from
    /// * `file` - The file to read
    /// * `xtea_keys` - The XTEA keys to use for decryption. If None, the file will not be decrypted
    pub fn read_named_group(
        &mut self,
        archive: u8,
        group: &str,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
    ) -> Result<Vec<u8>, CacheError> {
        Ok(self
            .archives
            .get_mut(&archive)
            .ok_or(CacheError::ArchiveNotFound(archive))?
            .read_named_group(djb2_hash(group), file, xtea_keys, self.store.as_ref())?)
    }
}
