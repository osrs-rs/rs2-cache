pub mod archive;
pub mod cache;
pub mod djb2;
pub mod ffi;
pub mod group;
pub mod js5_compression;
pub mod js5_index;
pub mod store;
mod xtea;

const MAX_GROUP_SIZE: usize = (1 << 24) - 1;
