use crate::store::Store;
use crc32fast::hash;
use osrs_bytes::WriteExt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChecksumTableError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store error: {0}")]
    Store(#[from] crate::store::StoreError),
}

pub struct ChecksumTable {
    entries: Vec<u32>,
}

impl ChecksumTable {
    pub fn write(&self) -> Result<(), ChecksumTableError> {
        let mut buf = Vec::new();

        for entry in &self.entries {
            buf.write_u32(*entry)?;
        }

        let mut checksum: u32 = 1234;
        for entry in &self.entries {
            checksum = (checksum << 1).wrapping_add(*entry)
        }

        Ok(())
    }

    pub fn create(store: &Box<dyn Store>) -> Result<ChecksumTable, ChecksumTableError> {
        let mut entries = Vec::new();
        let mut next_archive = 0;

        for archive in store.list(0)? {
            let entry = hash(&store.read(0, archive)?);

            for _ in next_archive..archive {
                //entries.push(0);
            }

            entries.push(entry);
            next_archive = archive + 1;
        }

        Ok(ChecksumTable { entries })
    }
}
