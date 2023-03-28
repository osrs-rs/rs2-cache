use self::{disk_store::DiskStore, flat_file_store::FlatFileStore};
use std::path::Path;

pub mod disk_store;
pub mod flat_file_store;

const DATA_PATH: &str = "main_file_cache.dat2";
const LEGACY_DATA_PATH: &str = "main_file_cache.dat2";

/// The store is responsible for reading and writing data of the various RS2 formats.
pub trait Store {
    fn list(&self, archive: u8) -> Vec<u32>;
    fn read(&self, archive: u8, group: u32) -> Vec<u8>;
}

pub fn store_open(path: &str) -> Box<dyn Store> {
    let has_data_file = Path::new(&path).join(DATA_PATH).exists();
    let has_legacy_data_file = Path::new(path).join(LEGACY_DATA_PATH).exists();

    if has_data_file || has_legacy_data_file {
        Box::new(DiskStore::open(path))
    } else {
        Box::new(FlatFileStore::open(path))
    }
}
