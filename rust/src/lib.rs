use bzip2::read::BzDecoder;
use flate2::bufread::GzDecoder;
use lzma_rs::{decompress, lzma_decompress_with_options};
use memmap2::Mmap;
use osrs_bytes::ReadExt;
use std::{
    cmp,
    collections::{BTreeMap, HashMap},
    ffi::CStr,
    fs::File,
    i32,
    io::{self, Cursor, Read},
    mem,
    os::raw::c_char,
    path::Path,
};
use thiserror::Error;

const EXTENDED_BLOCK_HEADER_SIZE: usize = 10;
const BLOCK_HEADER_SIZE: usize = 8;
const EXTENDED_BLOCK_DATA_SIZE: usize = 510;
const BLOCK_DATA_SIZE: usize = 512;
const MUSIC_ARCHIVE: u8 = 40;
const BLOCK_SIZE: usize = BLOCK_HEADER_SIZE + BLOCK_DATA_SIZE;
const INDEX_ENTRY_SIZE: usize = 6;

const COMPRESSION_TYPE_NONE: u8 = 0;
const COMPRESSION_TYPE_BZIP: u8 = 1;
const COMPRESSION_TYPE_GZIP: u8 = 2;
const COMPRESSION_TYPE_LZMA: u8 = 3;

const INDEX_PATH: &str = "main_file_cache.idx";
const DATA_PATH: &str = "main_file_cache.dat2";
const LEGACY_DATA_PATH: &str = "main_file_cache.dat2";
const MUSIC_DATA_PATH: &str = "main_file_cache.dat2m";
const UNPACKED_CACHE_SIZE_DEFAULT: usize = 1024;

const MAX_ARCHIVE: usize = 255;
const MAX_GROUP_SIZE: usize = (1 << 24) - 1;
const ARCHIVESET: usize = (1 << 24) - 1;

/// Implements the djb2 hash function for a string.
///
/// The djb2 hash function is a simple and efficient hash function that produces
/// good hash values for short strings.
pub fn djb2_hash<T: AsRef<str>>(string: T) -> u32 {
    // Convert the string to a byte slice.
    let string = string.as_ref().as_bytes();

    // Initialize the hash value to zero.
    let mut hash: u32 = 0;

    // Iterate over each byte in the string and update the hash value.
    for c in string {
        // Update the hash value using the djb2 algorithm.
        hash = ((hash << 5).wrapping_sub(hash)) + *c as u32;
    }

    // Return the final hash value.
    hash
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("data store disconnected")]
    Disconnect(#[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data store error")]
    Unknown,
}

struct IndexEntry {
    size: u32,
    block: u32,
}

pub struct DiskStore {
    root: String,
    data: Mmap,
    music_data: Option<Mmap>,
    indexes: HashMap<usize, Mmap>,
    legacy: bool,
}
pub struct FlatFileStore {}

impl FlatFileStore {
    pub fn open(path: &str) -> FlatFileStore {
        FlatFileStore {}
    }
}

// XTEA

const ROUNDS: u32 = 32;
const RATIO: u32 = 0x9E3779B9;

/// Enciphers the data with the given XTEA keys. Defaults to 32 rounds
fn encipher(data: &[u8], keys: &[u32; 4]) -> Vec<u8> {
    let blocks = data.len() / 8;
    let mut buf = data.to_vec();

    let mut index = 0;
    for _ in 0..blocks {
        let mut v0 = u32::from_be_bytes([
            data[index],
            data[index + 1],
            data[index + 2],
            data[index + 3],
        ]);
        let mut v1 = u32::from_be_bytes([
            data[index + 4],
            data[index + 5],
            data[index + 6],
            data[index + 7],
        ]);
        let mut sum = 0_u32;
        for _ in 0..ROUNDS {
            v0 = v0.wrapping_sub(
                (((v1 << 4) ^ (v1 >> 5)).wrapping_add(v1))
                    ^ (sum.wrapping_add(keys[(sum & 3) as usize])),
            );
            sum = sum.wrapping_sub(RATIO);
            v1 = v1.wrapping_sub(
                (((v0 << 4) ^ (v0 >> 5)).wrapping_add(v0))
                    ^ (sum.wrapping_add(keys[((sum >> 11) & 3) as usize])),
            );
        }
        buf[index..index + 4].copy_from_slice(&v0.to_be_bytes());
        buf[index + 4..index + 8].copy_from_slice(&v1.to_be_bytes());

        index += 8;
    }

    buf
}

/// Deciphers the data with the given XTEA keys. Defaults to 32 rounds.
fn decipher(data: &[u8], keys: &[u32; 4]) -> Vec<u8> {
    let blocks = data.len() / 8;
    let mut buf = data.to_vec();

    let mut index = 0;
    for _ in 0..blocks {
        let mut v0 =
            u32::from_be_bytes([buf[index], buf[index + 1], buf[index + 2], buf[index + 3]]);
        let mut v1 = u32::from_be_bytes([
            buf[index + 4],
            buf[index + 5],
            buf[index + 6],
            buf[index + 7],
        ]);
        let mut sum = ROUNDS.wrapping_mul(RATIO);
        for _ in 0..ROUNDS {
            v1 = v1.wrapping_sub(
                (((v0 << 4) ^ (v0 >> 5)).wrapping_add(v0))
                    ^ (sum.wrapping_add(keys[((sum >> 11) & 3) as usize])),
            );
            sum = sum.wrapping_sub(RATIO);
            v0 = v0.wrapping_sub(
                (((v1 << 4) ^ (v1 >> 5)).wrapping_add(v1))
                    ^ (sum.wrapping_add(keys[(sum & 3) as usize])),
            );
        }
        buf[index..index + 4].copy_from_slice(&v0.to_be_bytes());
        buf[index + 4..index + 8].copy_from_slice(&v1.to_be_bytes());

        index += 8;
    }

    buf
}

pub struct Js5Compression {}

impl Js5Compression {
    pub fn uncompress<T: AsRef<[u8]>>(input: T, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        let mut input_ref = input.as_ref();

        if input_ref.as_ref().len() < 5 {
            panic!("Missing header");
        }

        let type_id = input_ref.read_u8().unwrap();
        // TODO: Check if type_id is correct here and panic if not or just like throw an error and return here

        let len = input_ref.read_i32().unwrap();
        if len < 0 {
            panic!("Length is negative {len}");
        }

        if type_id == COMPRESSION_TYPE_NONE {
            if input_ref.len() < len as usize {
                panic!("Data truncated");
            }

            if let Some(xtea_keys) = xtea_keys {
                return decipher(input_ref, &xtea_keys);
            }

            return input_ref[..len as usize].to_vec();
        }

        let len_with_uncompressed_len = len + 4;
        if input_ref.len() < len_with_uncompressed_len as usize {
            panic!("Data truncated");
        }

        let plain_text = Self::decrypt(input_ref, len_with_uncompressed_len, xtea_keys);
        let mut plain_text_csr = Cursor::new(plain_text);

        let uncompressed_len = plain_text_csr.read_i32().unwrap();
        if uncompressed_len < 0 {
            panic!("Uncompressed length is negative: {uncompressed_len}");
        }

        // Copy bytes from the cursor to a buffer skipping over already read ones
        let mut plain_text =
            vec![0; plain_text_csr.get_ref().len() - plain_text_csr.position() as usize];

        plain_text_csr.read_exact(&mut plain_text).unwrap();

        // Skip version by using len
        let input_stream = &plain_text[..len as usize];

        match type_id {
            COMPRESSION_TYPE_BZIP => {
                decompress_archive_bzip2(input_stream, uncompressed_len as u32)
            }
            COMPRESSION_TYPE_GZIP => decompress_archive_gzip(input_stream, uncompressed_len as u32),
            COMPRESSION_TYPE_LZMA => decompress_archive_lzma(input_stream, uncompressed_len as u32),
            _ => panic!("Unknown compression type {type_id}"),
        }
    }

    fn decrypt<T: AsRef<[u8]>>(input: T, len: i32, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        if let Some(xtea_keys) = xtea_keys {
            decipher(input.as_ref(), &xtea_keys)
        } else {
            input.as_ref().to_vec()[..len as usize].to_vec()
        }
    }
}

// Decompress using bzip2
fn decompress_archive_bzip2<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut compressed_data = Vec::with_capacity(archive_data.as_ref().len() + 4);
    compressed_data.extend(b"BZh1");
    compressed_data.extend(archive_data.as_ref());

    let mut decompressor = BzDecoder::new(compressed_data.as_slice());

    decompressor.read_exact(&mut decompressed_data).unwrap();
    decompressed_data
}

// Decompress using gzip
fn decompress_archive_gzip<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut decompressor = GzDecoder::new(archive_data.as_ref());
    decompressor.read_exact(&mut decompressed_data).unwrap();

    decompressed_data
}

// Decompress using lzma
fn decompress_archive_lzma<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decomp: Vec<u8> = Vec::new();

    lzma_decompress_with_options(
        &mut archive_data.as_ref(),
        &mut decomp,
        &decompress::Options {
            unpacked_size: decompress::UnpackedSize::UseProvided(Some(decompressed_size as u64)),
            memlimit: None,
            allow_incomplete: false,
        },
    )
    .unwrap();

    decomp
}

impl Store for DiskStore {
    fn list(&self, archive: u8) -> Vec<u32> {
        let index = &self.indexes[&(archive as usize)];
        let mut index_csr = Cursor::new(index);

        let mut groups = Vec::new();
        let mut group = 0;
        while index_csr.read_u24().is_ok() {
            let block = index_csr.read_u24().unwrap();
            if block != 0 {
                groups.push(group);
            }

            group += 1;
        }

        groups
    }

    fn read(&self, archive: u8, group: u32) -> Vec<u8> {
        let entry = self.read_index_entry(archive, group).unwrap();
        if entry.block == 0 {
            panic!("file not found exception");
        }

        let mut buf = Vec::with_capacity(entry.size as usize);
        let data = self.get_data(archive);

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
                panic!("Group shorter than expected");
            }

            let pos = (block * BLOCK_SIZE as u32) as usize;
            if pos + header_size > self.data.len() {
                panic!("Next block is outside the data file");
            }

            let mut data_csr = Cursor::new(&data);
            data_csr.set_position(pos as u64);

            let actual_group = if extended {
                data_csr.read_u32().unwrap()
            } else {
                data_csr.read_u16().unwrap() as u32
            };
            let actual_num = data_csr.read_u16().unwrap();
            let next_block = data_csr.read_u24().unwrap();
            let actual_archive = data_csr.read_u8().unwrap() - (if self.legacy { 1 } else { 0 });

            if actual_group != group {
                panic!("Expecting group {group}, was {actual_group}");
            }
            if actual_num != num {
                panic!("Expecting block number {num}, was {actual_num}");
            }
            if actual_archive != archive {
                panic!("Expecting archive {archive}, was {actual_archive}");
            }

            // read data
            let len = cmp::min(entry.size as usize - buf.len(), data_size);
            buf.extend_from_slice(&data[pos + header_size..pos + header_size + len]);

            // advance to next block
            block = next_block;
            num += 1;
        }

        buf
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

impl DiskStore {
    pub fn open<P: AsRef<Path>>(path: P) -> DiskStore {
        let js5_data_path = Path::new(path.as_ref()).join(DATA_PATH);
        let legacy_data_path = Path::new(path.as_ref()).join(LEGACY_DATA_PATH);

        // We check for js5_data_path first as it takes precedence.
        let legacy = !js5_data_path.exists();

        let data_path = if legacy {
            legacy_data_path
        } else {
            js5_data_path
        };

        let data = unsafe { Mmap::map(&File::open(data_path).unwrap()) }.unwrap();

        let music_data_path = Path::new(path.as_ref()).join(MUSIC_DATA_PATH);
        let music_data = if music_data_path.exists() {
            Some(unsafe { Mmap::map(&File::open(music_data_path).unwrap()).unwrap() })
        } else {
            None
        };

        let mut archives = HashMap::new();
        for i in 0..MAX_ARCHIVE + 1 {
            let path = Path::new(path.as_ref()).join(format!("{INDEX_PATH}{i}"));
            if Path::new(&path).exists() {
                let index = unsafe { Mmap::map(&File::open(&path).unwrap()).unwrap() };
                archives.insert(i, index);
            }
        }

        DiskStore {
            root: String::from(path.as_ref().to_str().unwrap()),
            data,
            music_data,
            indexes: archives,
            legacy,
        }
    }

    fn get_data(&self, archive: u8) -> &Mmap {
        if archive == MUSIC_ARCHIVE && self.music_data.is_some() {
            self.music_data.as_ref().unwrap()
        } else {
            &self.data
        }
    }

    fn read_index_entry(&self, archive: u8, group: u32) -> Option<IndexEntry> {
        let index = &self.indexes[&(archive as usize)];

        let pos = (group as usize) * INDEX_ENTRY_SIZE;
        if pos + INDEX_ENTRY_SIZE > index.len() {
            return None;
        }

        let mut csr = Cursor::new(index);
        csr.set_position(pos as u64);

        let size = csr.read_u24().unwrap();
        let block = csr.read_u24().unwrap();

        Some(IndexEntry { size, block })
    }
}

trait Archive {
    fn is_dirty(&self) -> bool;
    fn read(
        &self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8>;
    fn read_named_group(
        &self,
        group: u32,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8>;
    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Unpacked;
    fn read_packed(&self, group: u32, store: &dyn Store) -> Vec<u8>;
    fn verify_compressed(&self, buf: &[u8], entry: &Js5IndexEntry);
    fn verify_uncompressed(&self, buf: &[u8], entry: &Js5IndexEntry);
}

pub struct Group {}

impl Group {
    pub fn unpack(input: Vec<u8>, group: &BTreeMap<u32, Js5IndexFile>) -> BTreeMap<u32, Vec<u8>> {
        if group.is_empty() {
            panic!("Group has no files")
        }

        if group.len() == 1 {
            let single_entry = group.keys().next().unwrap();
            let mut files = BTreeMap::new();
            files.insert(*single_entry, input);
            return files;
        }

        let mut input_reader = Cursor::new(&input);

        // Now begin going over the stripes
        let stripes = *input.last().unwrap();

        let mut data_index = input_reader.position() as i32;
        let trailer_index = input.len() - (stripes as usize * group.len() * 4) - 1;

        input_reader.set_position(trailer_index as u64);

        let mut lens = vec![0; group.len()];
        for _ in 0..stripes {
            let mut prev_len = 0;
            for j in lens.iter_mut() {
                prev_len += input_reader.read_i32().unwrap();
                *j += prev_len;
            }
        }

        input_reader.set_position(trailer_index as u64);

        let mut files = BTreeMap::new();
        for (i, x) in group.keys().enumerate() {
            files.insert(*x, Vec::with_capacity(lens[i] as usize));
        }

        for _ in 0..stripes {
            let mut prev_len = 0;
            for x in group.keys() {
                prev_len += input_reader.read_i32().unwrap();
                let dst = files.get_mut(x).unwrap();
                let cap = dst.capacity();
                dst.extend_from_slice(
                    &input[data_index as usize..(data_index + prev_len) as usize],
                );
                // Truncate to the correct length in case the buffer has
                // too much data pushed into it.
                // In OpenRS2 it a hard limit which is not supported in Rust
                dst.truncate(cap);

                data_index += prev_len;
            }
        }

        files
    }
}

/// The store is responsible for reading and writing data of the various RS2 formats.
pub trait Store {
    fn list(&self, archive: u8) -> Vec<u32>;
    fn read(&self, archive: u8, group: u32) -> Vec<u8>;
}

fn store_open(path: &str) -> Box<dyn Store> {
    let has_data_file = Path::new(&path).join(DATA_PATH).exists();
    let has_legacy_data_file = Path::new(path).join(LEGACY_DATA_PATH).exists();

    if has_data_file || has_legacy_data_file {
        Box::new(DiskStore::open(path))
    } else {
        Box::new(FlatFileStore::open(path))
    }
}
struct CacheArchive {
    is_dirty: bool,
    index: Js5Index,
    archive: u8,
    unpacked_cache: HashMap<u64, Unpacked>,
}

impl CacheArchive {
    pub fn testy() {}
}

struct Unpacked {
    dirty: bool,
    key: Option<[u32; 4]>,
    files: BTreeMap<u32, Vec<u8>>,
}

impl Unpacked {
    pub fn read(&self, file: u32) -> Vec<u8> {
        self.files.get(&file).unwrap().to_vec()
    }
}

impl Archive for CacheArchive {
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn read(&self, group: u32, file: u16, key: Option<[u32; 4]>, store: &dyn Store) -> Vec<u8> {
        let entry = self.index.groups.get(&group).unwrap();
        let unpacked = self.get_unpacked(entry, group, key, store);
        unpacked.read(file as u32)
    }

    fn read_named_group(
        &self,
        group_name_hash: u32,
        file: u16,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Vec<u8> {
        let entry_id = self.index.get_named(group_name_hash).unwrap();
        let entry = self.index.groups.get(&entry_id).unwrap();

        let unpacked = self.get_unpacked(entry, entry_id, key, store);
        unpacked.read(file as u32)
    }

    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u32,
        key: Option<[u32; 4]>,
        store: &dyn Store,
    ) -> Unpacked {
        // TODO: Handle unpacked cache aka cached version of the unpacked files

        let compressed = self.read_packed(entry_id, store);

        self.verify_compressed(&compressed, entry);

        let buf = Js5Compression::uncompress(compressed, key);

        self.verify_uncompressed(&buf, entry);

        let files = Group::unpack(buf, &self.index.groups.get(&entry_id).unwrap().files);

        //self.unpacked_cache.insert(123, unpacked);

        Unpacked {
            dirty: false,
            key,
            files,
        }
    }

    fn read_packed(&self, group: u32, store: &dyn Store) -> Vec<u8> {
        store.read(self.archive, group)
    }

    // TODO: Implement
    fn verify_compressed(&self, buf: &[u8], entry: &Js5IndexEntry) {}

    // TODO: Implement
    fn verify_uncompressed(&self, buf: &[u8], entry: &Js5IndexEntry) {}
}

pub struct Cache {
    /// Store
    store: Box<dyn Store>,

    /// Archives
    archives: HashMap<u8, CacheArchive>,

    /// Unpacked cache size
    unpacked_cache_size: usize,
}

pub enum Js5Protocol {
    Original = 5,
    Versioned = 6,
    Smart = 7,
}

enum Js5IndexFlags {
    Names = 0x1,
    Digests = 0x2,
    Lengths = 0x4,
    UncompressedChecksums = 0x8,
}

#[derive(Debug, PartialEq)]
pub struct Js5IndexFile {
    pub name_hash: i32,
}

#[derive(Debug, PartialEq)]
pub struct Js5IndexEntry {
    pub name_hash: i32,
    pub version: u32,
    pub checksum: u32,
    pub uncompressed_checksum: u32,
    pub length: u32,
    pub uncompressed_length: u32,
    pub digest: Vec<u8>,
    pub capacity: u32,
    pub files: BTreeMap<u32, Js5IndexFile>,
}

#[derive(Debug, PartialEq)]
pub struct Js5Index {
    pub protocol: u8,
    pub version: i32,
    pub has_names: bool,
    pub has_digests: bool,
    pub has_lengths: bool,
    pub has_uncompressed_checksums: bool,
    pub groups: BTreeMap<u32, Js5IndexEntry>,
    pub name_hash_table: HashMap<u32, u32>,
}

impl Js5Index {
    pub fn read<T: AsRef<[u8]>>(buf: T) -> Js5Index {
        let mut buf_ref = buf.as_ref();

        let protocol = buf_ref.read_u8().unwrap();

        let read_func = if protocol >= Js5Protocol::Smart as u8 {
            |v: &mut &[u8]| -> u32 { v.read_u32_smart().unwrap() }
        } else {
            |v: &mut &[u8]| -> u32 { v.read_u16().unwrap() as u32 }
        };

        let version = if protocol >= Js5Protocol::Versioned as u8 {
            buf_ref.read_i32().unwrap()
        } else {
            0
        };
        let flags = buf_ref.read_u8().unwrap();
        let size = read_func(&mut buf_ref);

        // Create Js5Index
        let mut index = Js5Index {
            protocol,
            version,
            has_names: (flags & Js5IndexFlags::Names as u8) != 0,
            has_digests: (flags & Js5IndexFlags::Digests as u8) != 0,
            has_lengths: (flags & Js5IndexFlags::Lengths as u8) != 0,
            has_uncompressed_checksums: (flags & Js5IndexFlags::UncompressedChecksums as u8) != 0,
            groups: BTreeMap::new(),
            name_hash_table: HashMap::new(),
        };

        // Begin creating the groups
        let mut prev_group_id = 0;
        (0..size).for_each(|_| {
            prev_group_id += read_func(&mut buf_ref);
            index.groups.insert(
                prev_group_id,
                Js5IndexEntry {
                    name_hash: -1,
                    version: 0,
                    checksum: 0,
                    uncompressed_checksum: 0,
                    length: 0,
                    uncompressed_length: 0,
                    digest: Vec::new(),
                    capacity: 0,
                    files: BTreeMap::new(),
                },
            );
        });

        if index.has_names {
            for (id, group) in index.groups.iter_mut() {
                group.name_hash = buf_ref.read_i32().unwrap();
                index.name_hash_table.insert(group.name_hash as u32, *id);
            }
        }

        for group in index.groups.values_mut() {
            group.checksum = buf_ref.read_u32().unwrap();
        }

        if index.has_uncompressed_checksums {
            for group in index.groups.values_mut() {
                group.uncompressed_checksum = buf_ref.read_u32().unwrap();
            }
        }

        if index.has_digests {
            for group in index.groups.values_mut() {
                let digest_bits = 512;
                let digest_bytes = digest_bits >> 3;
                let mut digest = vec![0; digest_bytes];
                buf_ref.read_exact(&mut digest).unwrap();
                group.digest.extend(&digest);
            }
        }

        if index.has_lengths {
            for group in index.groups.values_mut() {
                group.length = buf_ref.read_u32().unwrap();
                group.uncompressed_length = buf_ref.read_u32().unwrap();
            }
        }

        for group in index.groups.values_mut() {
            group.version = buf_ref.read_u32().unwrap();
        }

        let group_sizes: Vec<u32> = (0..size).map(|_| read_func(&mut buf_ref)).collect();

        for (i, group) in index.groups.values_mut().enumerate() {
            let group_size = group_sizes[i];

            let mut prev_file_id = 0;
            (0..group_size).for_each(|_| {
                prev_file_id += read_func(&mut buf_ref);
                group
                    .files
                    .insert(prev_file_id, Js5IndexFile { name_hash: -1 });
            });
        }

        if index.has_names {
            for group in index.groups.values_mut() {
                for file in group.files.values_mut() {
                    file.name_hash = buf_ref.read_i32().unwrap();
                }
            }
        }

        index
    }

    fn get_named(&self, name_hash: u32) -> Option<u32> {
        self.name_hash_table.get(&name_hash).copied()
    }
}

impl Cache {
    pub fn open(input_path: &str) -> io::Result<Cache> {
        Self::open_with_store(store_open(input_path))
    }

    pub fn open_with_store(store: Box<dyn Store>) -> io::Result<Cache> {
        let mut cache = Self {
            store,
            archives: HashMap::new(),
            unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
        };
        cache.init();

        // Return the Cache struct
        Ok(cache)
    }

    fn init(&mut self) {
        for archive in self.store.list(ARCHIVESET as u8) {
            let compressed = self.store.read(ARCHIVESET as u8, archive);

            let buf = Js5Compression::uncompress(compressed, None);

            let js5_index = Js5Index::read(buf);

            let cache_archive = CacheArchive {
                is_dirty: false,
                index: js5_index,
                archive: archive as u8,
                unpacked_cache: HashMap::new(),
            };

            self.archives.insert(archive as u8, cache_archive);
        }
    }

    /// Read a file from the cache
    ///
    /// # Arguments
    ///
    /// * `archive` - The archive to read from
    /// * `group` - The group to read from
    /// * `file` - The file to read
    /// * `xtea_keys` - The XTEA keys to use for decryption. If None, the file will not be decrypted
    pub fn read(&self, archive: u8, group: u32, file: u16, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        self.archives[&archive].read(group, file, xtea_keys, self.store.as_ref())
    }

    pub fn read_named_group(
        &self,
        archive: u8,
        group: &str,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
    ) -> Vec<u8> {
        self.archives[&archive].read_named_group(
            djb2_hash(group),
            file,
            xtea_keys,
            self.store.as_ref(),
        )
    }
}
/*
#[no_mangle]
pub unsafe extern "C" fn cache_create(cache_ptr: *mut Cache, archive: u32) {}
#[no_mangle]
pub unsafe extern "C" fn cache_capacity(cache_ptr: *mut Cache, archive: u32) {}
*/

/// Open a cache at the given path
///
/// # Arguments
///
/// * `path` - The path to the cache
///
/// # Returns
///
/// A pointer to the cache
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer
#[no_mangle]
pub unsafe extern "C" fn cache_open(path: *const c_char) -> *mut Cache {
    // Get CStr
    let path_cstr = CStr::from_ptr(path);

    // Convert to Rust str
    let path_str = path_cstr.to_str().unwrap();

    // Open the cache
    let cache = Cache::open(path_str).expect("failed to open cache");

    // Return the cache as a Box
    Box::into_raw(Box::new(cache))
}

/// Read a file from the cache
///
/// # Arguments
///
/// * `cache_ptr` - The cache to read from
/// * `archive` - The archive to read from
/// * `group` - The group to read from
/// * `file` - The file to read
/// * `xtea_keys` - The optional XTEA keys to use for decryption
/// * `out_len` - The length of the returned buffer
///
/// # Returns
///
/// The function returns a pointer to the buffer containing the file data, where the length is stored in the `out_len` variable.
/// The caller is responsible for freeing the buffer using the function `cache_free`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn cache_read(
    cache_ptr: *mut Cache,
    archive: u8,
    group: u32,
    file: u16,
    xtea_keys_arg: *const [u32; 4],
    out_len: *mut u32,
) -> *mut u8 {
    // Dereference the cache
    let cache = &*cache_ptr;

    // Dereference the xtea keys if not null
    let mut xtea_keys = None;
    if !xtea_keys_arg.is_null() {
        xtea_keys = Some(*xtea_keys_arg);
    }

    // Call the read function
    let mut buf = cache.read(archive, group, file, xtea_keys);

    let data = buf.as_mut_ptr();
    *out_len = buf.len() as u32;
    mem::forget(buf);
    data
}

/// Read a named group from the cache
///
/// # Arguments
///
/// * `cache_ptr` - The cache to read from
/// * `archive` - The archive to read from
/// * `group` - The group to read from
/// * `file` - The file to read
/// * `xtea_keys` - The optional XTEA keys to use for decryption
/// * `out_len` - The length of the returned buffer
///
/// # Returns
///
/// The function returns a pointer to the buffer containing the file data, where the length is stored in the `out_len` variable.
/// The caller is responsible for freeing the buffer using the function `cache_free`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn cache_read_named_group(
    cache_ptr: *mut Cache,
    archive: u8,
    group: *const c_char,
    file: u16,
    xtea_keys_arg: *const [u32; 4],
    out_len: *mut u32,
) -> *mut u8 {
    // Dereference the cache
    let cache = &*cache_ptr;

    // Dereference the xtea keys if not null
    let mut xtea_keys = None;
    if !xtea_keys_arg.is_null() {
        xtea_keys = Some(*xtea_keys_arg);
    }

    let group_str = CStr::from_ptr(group).to_str().unwrap();

    // Call the read function
    let mut buf = cache.read_named_group(archive, group_str, file, xtea_keys);

    let data = buf.as_mut_ptr();
    *out_len = buf.len() as u32;
    mem::forget(buf);
    data
}

/// Free a buffer returned by cache read functions
///
/// # Arguments
///
/// * `buffer` - The buffer to free
///
/// # Safety
///
/// This function is unsafe because it dereferences the pointer to the buffer.
/// The caller must ensure that the pointer is valid.
#[no_mangle]
pub unsafe extern "C" fn cache_free(buffer: *mut u8) {
    // If the buffer is not null, drop the Vec
    if !buffer.is_null() {
        drop(Vec::from_raw_parts(buffer, 0, 0))
    }
}

/*
#[no_mangle]
pub unsafe extern "C" fn cache_write(cache_ptr: *mut Cache, archive: u32) {}
#[no_mangle]
pub unsafe extern "C" fn cache_remove(cache_ptr: *mut Cache, archive: u32) {}
*/

/// Close a cache
///
/// # Arguments
///
/// * `cache_ptr` - The cache to close
///
/// # Safety
///
/// This function is unsafe because it dereferences the pointer to the cache.
/// The caller must ensure that the pointer is valid.
/// The caller must also ensure that the cache is not used after it has been closed.
#[no_mangle]
pub unsafe extern "C" fn cache_close(cache_ptr: *mut Cache) {
    // If the pointer to the cache is not null, drop the box
    if !cache_ptr.is_null() {
        drop(Box::from_raw(cache_ptr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_djb2_hashing() {
        let hashed_value = djb2_hash("m50_50");
        let assert_val = -1123920270;

        assert_eq!(hashed_value, assert_val as u32);
    }

    /*#[test]
    fn party_hat_test() {
        let cache = Cache::open("data/cache").unwrap();
        fs::write("blue_partyhat.dat", cache.read(2, 10, 1042, None));
    }*/
}
