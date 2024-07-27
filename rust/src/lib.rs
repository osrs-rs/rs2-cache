use archive::cache_archive::CacheArchive;
use group::GroupError;
use std::collections::HashMap;
use store::Store;

mod archive;
pub mod cache;
pub mod checksumtable;
mod djb2;
mod ffi;
mod group;
mod js5_compression;
mod js5_index;
pub mod js5_masterindex;
pub mod store;
mod xtea;

const _MAX_GROUP_SIZE: usize = (1 << 24) - 1;

pub struct Cache {
    /// Store
    pub store: Box<dyn Store + Send>,

    /// Archives
    archives: HashMap<u8, CacheArchive>,

    /// Unpacked cache size
    _unpacked_cache_size: usize,
}
