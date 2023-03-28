use super::Store;

pub struct FlatFileStore {}

impl FlatFileStore {
    pub fn open(path: &str) -> FlatFileStore {
        FlatFileStore {}
    }
}

impl Store for FlatFileStore {
    fn list(&self, archive: u8) -> Vec<u32> {
        todo!()
    }

    fn read(&self, archive: u8, group: u32) -> Vec<u8> {
        todo!()
    }
}
