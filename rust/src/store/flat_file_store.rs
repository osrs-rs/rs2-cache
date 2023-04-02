use super::{Store, StoreError};

pub struct FlatFileStore {}

impl FlatFileStore {
    pub fn open(_path: &str) -> Result<FlatFileStore, StoreError> {
        Ok(FlatFileStore {})
    }
}

impl Store for FlatFileStore {
    fn list(&self, _archive: u8) -> Result<Vec<u32>, StoreError> {
        todo!()
    }

    fn read(&self, _archive: u8, _group: u32) -> Result<Vec<u8>, StoreError> {
        todo!()
    }
}
