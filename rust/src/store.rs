use self::{disk_store::DiskStore, flat_file_store::FlatFileStore};
use std::path::Path;
use thiserror::Error;

pub mod disk_store;
pub mod flat_file_store;

const DATA_PATH: &str = "main_file_cache.dat2";
const LEGACY_DATA_PATH: &str = "main_file_cache.dat2";
pub const ARCHIVESET: u8 = 255;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("disk_store error {0}")]
    DiskStore(#[from] disk_store::DiskStoreError),
    #[error("group shorter than expected")]
    GroupTooShort,
    #[error("next block is outside the data file")]
    NextBlockOutsideDataFile,
    #[error("expecting group {0}, was {1}")]
    GroupMismatch(u32, u32),
    #[error("expecting block number {0}, was {1}")]
    BlockMismatch(u16, u16),
    #[error("expecting archive {0}, was {1}")]
    ArchiveMismatch(u8, u8),
}

/// The store is responsible for reading and writing data of the various RS2 formats.
pub trait Store {
    fn list(&self, archive: u8) -> Result<Vec<u32>, StoreError>;
    fn read(&self, archive: u8, group: u32) -> Result<Vec<u8>, StoreError>;
}

pub fn store_open(path: &str) -> Result<Box<dyn Store + Send + Sync>, StoreError> {
    let has_data_file = Path::new(&path).join(DATA_PATH).exists();
    let has_legacy_data_file = Path::new(path).join(LEGACY_DATA_PATH).exists();

    if has_data_file || has_legacy_data_file {
        Ok(Box::new(DiskStore::open(path)?))
    } else {
        Ok(Box::new(FlatFileStore::open(path)?))
    }
}
