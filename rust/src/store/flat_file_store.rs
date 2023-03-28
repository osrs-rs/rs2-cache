use super::Store;

pub struct FlatFileStore {}

impl FlatFileStore {
    pub fn open(_path: &str) -> FlatFileStore {
        FlatFileStore {}
    }
}

impl Store for FlatFileStore {
    fn list(&self, _archive: u8) -> Vec<u32> {
        todo!()
    }

    fn read(&self, _archive: u8, _group: u32) -> Vec<u8> {
        todo!()
    }
}
