use super::{Store, StoreError, DATA_PATH, LEGACY_DATA_PATH};
use memmap2::Mmap;
use osrs_bytes::ReadExt;
use std::{cmp, collections::HashMap, fs::File, io::Cursor, path::Path};
use thiserror::Error;

const EXTENDED_BLOCK_HEADER_SIZE: usize = 10;
const BLOCK_HEADER_SIZE: usize = 8;
const EXTENDED_BLOCK_DATA_SIZE: usize = 510;
const BLOCK_DATA_SIZE: usize = 512;
const MUSIC_ARCHIVE: u8 = 40;
const BLOCK_SIZE: usize = BLOCK_HEADER_SIZE + BLOCK_DATA_SIZE;
const INDEX_ENTRY_SIZE: usize = 6;

const INDEX_PATH: &str = "main_file_cache.idx";
const MUSIC_DATA_PATH: &str = "main_file_cache.dat2m";

const MAX_ARCHIVE: usize = 255;

#[derive(Error, Debug)]
pub enum DiskStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed converting root to string")]
    RootToString,
    #[error("failed getting music data")]
    MusicData,
    #[error("file not found")]
    FileNotFound,
}

struct IndexEntry {
    size: u32,
    block: u32,
}

pub struct DiskStore {
    _root: String,
    data: Mmap,
    music_data: Option<Mmap>,
    indexes: HashMap<usize, Mmap>,
    legacy: bool,
}

impl DiskStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<DiskStore, DiskStoreError> {
        let js5_data_path = Path::new(path.as_ref()).join(DATA_PATH);
        let legacy_data_path = Path::new(path.as_ref()).join(LEGACY_DATA_PATH);

        // We check for js5_data_path first as it takes precedence.
        let legacy = !js5_data_path.exists();

        let data_path = if legacy {
            legacy_data_path
        } else {
            js5_data_path
        };

        let data = unsafe { Mmap::map(&File::open(data_path)?) }?;

        let music_data_path = Path::new(path.as_ref()).join(MUSIC_DATA_PATH);
        let music_data = if music_data_path.exists() {
            Some(unsafe { Mmap::map(&File::open(music_data_path)?)? })
        } else {
            None
        };

        let mut archives = HashMap::new();
        for i in 0..MAX_ARCHIVE + 1 {
            let path = Path::new(path.as_ref()).join(format!("{INDEX_PATH}{i}"));
            if Path::new(&path).exists() {
                let index = unsafe { Mmap::map(&File::open(&path)?)? };
                archives.insert(i, index);
            }
        }

        Ok(DiskStore {
            _root: String::from(path.as_ref().to_str().ok_or(DiskStoreError::RootToString)?),
            data,
            music_data,
            indexes: archives,
            legacy,
        })
    }

    fn get_data(&self, archive: u8) -> Result<&Mmap, DiskStoreError> {
        if archive == MUSIC_ARCHIVE && self.music_data.is_some() {
            Ok(self.music_data.as_ref().ok_or(DiskStoreError::MusicData)?)
        } else {
            Ok(&self.data)
        }
    }

    fn read_index_entry(&self, archive: u8, group: u32) -> Result<IndexEntry, DiskStoreError> {
        let index = &self.indexes[&(archive as usize)];

        let pos = (group as usize) * INDEX_ENTRY_SIZE;
        if pos + INDEX_ENTRY_SIZE > index.len() {
            return Err(DiskStoreError::FileNotFound);
        }

        let mut csr = Cursor::new(index);
        csr.set_position(pos as u64);

        let size = csr.read_u24()?;
        let block = csr.read_u24()?;

        Ok(IndexEntry { size, block })
    }
}

impl Store for DiskStore {
    fn list(&self, archive: u8) -> Result<Vec<u32>, StoreError> {
        let index = &self.indexes[&(archive as usize)];
        let mut index_csr = Cursor::new(index);

        let mut groups = Vec::new();
        let mut group = 0;
        while index_csr.read_u24().is_ok() {
            let block = index_csr.read_u24()?;
            if block != 0 {
                groups.push(group);
            }

            group += 1;
        }

        Ok(groups)
    }

    fn read(&self, archive: u8, group: u32) -> Result<Vec<u8>, StoreError> {
        let entry = self.read_index_entry(archive, group)?;
        if entry.block == 0 {
            return Err(StoreError::GroupTooShort);
        }

        let mut buf = Vec::with_capacity(entry.size as usize);
        let data = self.get_data(archive)?;

        let extended = group >= 65536;
        let header_size = if extended {
            EXTENDED_BLOCK_HEADER_SIZE
        } else {
            BLOCK_HEADER_SIZE
        };
        let data_size = if extended {
            EXTENDED_BLOCK_DATA_SIZE
        } else {
            BLOCK_DATA_SIZE
        };

        let mut block = entry.block;
        let mut num = 0;

        while buf.len() < entry.size as usize {
            if block == 0 {
                return Err(StoreError::GroupTooShort);
            }

            let pos = (block * BLOCK_SIZE as u32) as usize;
            if pos + header_size > self.data.len() {
                return Err(StoreError::NextBlockOutsideDataFile);
            }

            let mut data_csr = Cursor::new(&data);
            data_csr.set_position(pos as u64);

            let actual_group = if extended {
                data_csr.read_u32()?
            } else {
                data_csr.read_u16()? as u32
            };
            let actual_num = data_csr.read_u16()?;
            let next_block = data_csr.read_u24()?;
            let actual_archive = data_csr.read_u8()? - (if self.legacy { 1 } else { 0 });

            if actual_group != group {
                return Err(StoreError::GroupMismatch(group, actual_group));
            }
            if actual_num != num {
                return Err(StoreError::BlockMismatch(num, actual_num));
            }
            if actual_archive != archive {
                return Err(StoreError::ArchiveMismatch(archive, actual_archive));
            }

            // read data
            let len = cmp::min(entry.size as usize - buf.len(), data_size);
            buf.extend_from_slice(&data[pos + header_size..pos + header_size + len]);

            // advance to next block
            block = next_block;
            num += 1;
        }

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_groups() {
        read_test("single-block", |store| {
            assert_eq!(vec![1], store.list(255).unwrap());
        });
        read_test("fragmented", |store| {
            assert_eq!(vec![0, 1], store.list(255).unwrap());
        });
        read_test("single-block-extended", |store| {
            assert_eq!(vec![65536], store.list(255).unwrap());
        });
    }

    // TODO: Handle this error
    /*#[test]
    fn test_list_non_existent() {
        read_test("empty", |store| {
            assert_eq!(vec![1], store.list(255));
        });
    }*/

    #[test]
    fn test_read_single_block() {
        read_test("single-block", |store| {
            let actual = store.read(255, 1).unwrap();
            let expected = "OpenRS2".as_bytes();
            assert_eq!(expected, actual);
        });
    }

    #[test]
    fn test_read_single_block_extended() {
        read_test("single-block-extended", |store| {
            let actual = store.read(255, 65536).unwrap();
            let expected = "OpenRS2".as_bytes();
            assert_eq!(expected, actual);
        });
    }

    #[test]
    fn test_read_two_blocks() {
        read_test("two-blocks", |store| {
            let actual = store.read(255, 1).unwrap();
            let expected = "OpenRS2".repeat(100).into_bytes();
            assert_eq!(expected, actual);
        });
    }

    #[test]
    fn test_read_two_blocks_extended() {
        read_test("two-blocks-extended", |store| {
            let actual = store.read(255, 65536).unwrap();
            let expected = "OpenRS2".repeat(100).into_bytes();
            assert_eq!(expected, actual);
        });
    }

    #[test]
    fn test_read_multiple_blocks() {
        read_test("multiple-blocks", |store| {
            let actual = store.read(255, 1).unwrap();
            let expected = "OpenRS2".repeat(1000).into_bytes();
            assert_eq!(expected, actual);
        });
    }

    #[test]
    fn test_read_multiple_blocks_extended() {
        read_test("multiple-blocks-extended", |store| {
            let actual = store.read(255, 65536).unwrap();
            let expected = "OpenRS2".repeat(1000).into_bytes();
            assert_eq!(expected, actual);
        });
    }

    // TODO: Error handling here, simply follow the trace of error and handle accordingly
    /*#[test]
    fn test_read_non_existent() {
        read_test("single-block", |store| {
            store.read(0, 0);
            store.read(255, 0);
            store.read(255, 2);
        });
    }*/

    #[test]
    fn test_read_fragmented() {
        read_test("fragmented", |store| {
            let actual = store.read(255, 1).unwrap();
            let expected = "OpenRS2".repeat(100).into_bytes();
            assert_eq!(expected, actual);
        });
    }

    fn read_test<P, F>(p: P, f: F)
    where
        P: AsRef<Path>,
        F: FnOnce(DiskStore),
    {
        f(DiskStore::open(Path::new("tests/data/disk-store").join(p)).unwrap())
    }
}
