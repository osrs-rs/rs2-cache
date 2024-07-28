use crate::{
    js5_compression::Js5CompressionError,
    js5_index::Js5IndexEntry,
    js5_index::Js5IndexError,
    store::{Store, StoreError},
    GroupError,
};
use std::collections::BTreeMap;
use thiserror::Error;

pub mod cache_archive;

// TODO: Move a lot of these to the CacheArchive error and then propagate back to ArchiveError as a "CacheArchiveError" as this is really bad right now
#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("Js5CompressionError: {0}")]
    Js5CompressionError(#[from] Js5CompressionError),
    #[error("Js5IndexError: {0}")]
    JS5IndexError(#[from] Js5IndexError),
    #[error("UnpackedError: {0}")]
    UnpackedError(#[from] UnpackedError),
    #[error("GroupError: {0}")]
    GroupError(#[from] GroupError),
    #[error("StoreError: {0}")]
    StoreError(#[from] StoreError),
    #[error("failed getting Js5Index group {0}")]
    GroupNotFound(u32),
}

pub trait Archive {
    fn is_dirty(&self) -> bool;
    fn read(
        &mut self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<Vec<u8>, ArchiveError>;
    fn read_named_group(
        &mut self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<Vec<u8>, ArchiveError>;
    fn get_unpacked(
        &mut self,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Result<&Unpacked, ArchiveError>;
    fn read_packed(&self, group: u32, store: &dyn Store) -> Result<Vec<u8>, ArchiveError>;
    fn verify_compressed(&self, buf: &[u8], entry: &Js5IndexEntry);
    fn verify_uncompressed(&self, buf: &[u8], entry: &Js5IndexEntry);
    fn write<T: AsRef<[u8]>>(
        &mut self,
        group: u32,
        file: u16,
        buf: T,
        key: Option<[u32; 4]>,
    ) -> Result<(), ArchiveError>;
}

#[derive(Error, Debug)]
pub enum UnpackedError {
    #[error("failed getting file {0} from unpacked cache")]
    FileNotFound(u32),
}

pub struct Unpacked {
    _dirty: bool,
    _key: Option<[u32; 4]>,
    files: BTreeMap<u32, Vec<u8>>,
}

impl Unpacked {
    pub fn read(&self, file: u32) -> Result<Vec<u8>, UnpackedError> {
        Ok(self
            .files
            .get(&file)
            .ok_or(UnpackedError::FileNotFound(file))?
            .to_vec())
    }
}
