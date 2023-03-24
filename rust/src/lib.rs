use byteorder::{ByteOrder, BE};
use bzip2::read::BzDecoder;
use extended_tea::XTEA;
use flate2::bufread::GzDecoder;
use memmap2::Mmap;
use osrs_bytes::ReadExt;
use std::{
    cmp,
    collections::{BTreeMap, HashMap},
    ffi::CStr,
    fs::{self, File},
    i32,
    io::{self, Cursor, Read, Seek, SeekFrom},
    mem,
    os::raw::c_char,
    path::Path,
};
use thiserror::Error;
use tracing::trace;

/// Implements the djb2 hash function for a string.
///
/// The djb2 hash function is a simple and efficient hash function that produces
/// good hash values for short strings.
fn djb2_hash<T: AsRef<str>>(string: T) -> u32 {
    // Convert the string to a byte slice.
    let string = string.as_ref().as_bytes();

    // Initialize the hash value to zero.
    let mut hash: u32 = 0;

    // Iterate over each byte in the string and update the hash value.
    for char in string {
        // Update the hash value using the djb2 algorithm.
        hash = *char as u32 + ((hash << 5).wrapping_sub(hash));
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

struct DiskStore {
    root: String,
    data: Mmap,
    music_data: Option<Mmap>,
    indexes: HashMap<usize, Mmap>,
    legacy: bool,
}
struct FlatFileStore {}

impl FlatFileStore {
    pub fn open(path: &str) -> FlatFileStore {
        FlatFileStore {}
    }
}

struct Js5Compression {}

impl Js5Compression {
    fn uncompress<T: AsRef<[u8]>>(input: T, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        let mut input_ref = input.as_ref();

        if input_ref.as_ref().len() < 5 {
            panic!("Missing header");
        }

        let type_id = input_ref.read_u8().unwrap();

        let len = input_ref.read_i32().unwrap();
        if len < 0 {
            panic!("Length is negative {}", len);
        }

        if type_id == COMPRESSION_TYPE_NONE {
            if input_ref.len() < len as usize {
                panic!("Data truncated");
            }

            if let Some(xtea_keys) = xtea_keys {
                let xtea = XTEA::new(&xtea_keys);

                let mut output = vec![0; len as usize];
                xtea.decipher_u8slice::<BE>(input_ref, &mut output);
                return output;
            }

            return input_ref.to_vec();
        }

        let len_with_uncompressed_len = len + 4;
        if input_ref.len() < len_with_uncompressed_len as usize {
            panic!("Data truncated");
        }

        let plain_text = Self::decrypt(input_ref, len, xtea_keys);
        let mut plain_text_csr = Cursor::new(plain_text);

        let uncompressed_len = plain_text_csr.read_i32().unwrap();
        if uncompressed_len < 0 {
            panic!("Uncompressed length is negative: {}", uncompressed_len);
        }

        // Copy bytes from the cursor to a buffer skipping over already read ones
        let mut plain_text =
            vec![0; plain_text_csr.get_ref().len() - plain_text_csr.position() as usize];
        plain_text_csr.read_exact(&mut plain_text).unwrap();

        match type_id {
            COMPRESSION_TYPE_BZIP => {
                decompress_archive_bzip2_2(plain_text, uncompressed_len as u32)
            }
            COMPRESSION_TYPE_GZIP => decompress_archive_gzip_2(plain_text, uncompressed_len as u32),
            _ => panic!("Unknown compression type {}", type_id),
        }
    }

    fn decrypt<T: AsRef<[u8]>>(input: T, len: i32, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        if let Some(xtea_keys) = xtea_keys {
            let xtea = XTEA::new(&xtea_keys);

            let mut output = vec![0; len as usize];
            xtea.decipher_u8slice::<BE>(input.as_ref(), &mut output);
            output
        } else {
            input.as_ref().to_vec()[..len as usize].to_vec()
        }
    }
}

// Decompress using bzip2
fn decompress_archive_bzip2_2<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut compressed_data = Vec::with_capacity(archive_data.as_ref().len() + 4);
    compressed_data.extend(b"BZh1");
    compressed_data.extend(archive_data.as_ref());

    let mut decompressor = BzDecoder::new(compressed_data.as_slice());

    decompressor.read_exact(&mut decompressed_data).unwrap();
    decompressed_data
}

// Decompress using gzip
fn decompress_archive_gzip_2<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    // Skip the first 9 bytes of the archive data to get to the gzip header
    let mut decompressor = GzDecoder::new(archive_data.as_ref());
    decompressor.read_exact(&mut decompressed_data).unwrap();

    decompressed_data
}

const EXTENDED_BLOCK_HEADER_SIZE: usize = 10;
const BLOCK_HEADER_SIZE: usize = 8;
const EXTENDED_BLOCK_DATA_SIZE: usize = 510;
const BLOCK_DATA_SIZE: usize = 512;
const MUSIC_ARCHIVE: u8 = 40;

const BLOCK_SIZE: usize = BLOCK_HEADER_SIZE + BLOCK_DATA_SIZE;

const INDEX_ENTRY_SIZE: usize = 6;

impl Store for DiskStore {
    fn list(&self, archive: u8) -> Vec<u32> {
        let index = &self.indexes[&(archive as usize)];
        let mut index_csr = Cursor::new(index);

        let mut groups = Vec::new();
        let mut group = 0;
        while let Ok(_) = index_csr.read_u24() {
            let block = index_csr.read_u24().unwrap();
            if block != 0 {
                groups.push(group);
            }

            group += 1;
        }

        trace!("list archive {} -> {:?}", archive, groups);
        groups
    }

    fn read(&self, archive: u8, group: u16) -> Vec<u8> {
        let entry = self.read_index_entry(archive, group).unwrap();
        if entry.block == 0 {
            panic!("file not found exception");
        }

        let mut buf = Vec::with_capacity(entry.size as usize);
        let data = self.get_data(archive);

        let extended = group as u32 >= 65536;
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
            let actual_archive =
                (data_csr.read_u8().unwrap() - (if self.legacy { 1 } else { 0 })) & 0xFF;

            if actual_group != group as u32 {
                panic!("Expecting group {}, was {}", group, actual_group);
            }
            if actual_num != num {
                panic!("Expecting block number {}, was {}", num, actual_num);
            }
            if actual_archive != archive {
                panic!("Expecting archive {}, was {}", archive, actual_archive);
            }

            // read data
            let bytes_to_read = cmp::min(entry.size as usize - buf.len(), data_size);
            buf.extend_from_slice(&data[pos + header_size..pos + header_size + bytes_to_read]);

            // advance to next block
            block = next_block;
            num += 1;
        }

        //fs::write(format!("archive{}_group{}", archive, group), &buf).unwrap();

        buf
    }
}

impl Store for FlatFileStore {
    fn list(&self, archive: u8) -> Vec<u32> {
        todo!()
    }

    fn read(&self, archive: u8, group: u16) -> Vec<u8> {
        todo!()
    }
}

enum StorageType {
    Disk(DiskStore),
    FlatFile(FlatFileStore),
}

impl DiskStore {
    pub fn open(path: &str) -> DiskStore {
        let js5_data_path = Path::new(path).join(DATA_PATH);
        let legacy_data_path = Path::new(path).join(LEGACY_DATA_PATH);

        // We check for js5_data_path first as it takes precedence.
        let legacy = !js5_data_path.exists();

        let data_path = if legacy {
            legacy_data_path
        } else {
            js5_data_path
        };

        let data = unsafe { Mmap::map(&File::open(data_path).unwrap()) }.unwrap();

        let music_data_path = Path::new(path).join(MUSIC_DATA_PATH);
        let music_data = if music_data_path.exists() {
            Some(unsafe { Mmap::map(&File::open(music_data_path).unwrap()).unwrap() })
        } else {
            None
        };

        let mut archives = HashMap::new();
        for i in 0..MAX_ARCHIVE + 1 {
            let path = Path::new(path).join(format!("{}{}", INDEX_PATH, i));
            if Path::new(&path).exists() {
                let index = unsafe { Mmap::map(&File::open(&path).unwrap()).unwrap() };
                archives.insert(i, index);
            }
        }

        DiskStore {
            root: path.to_string(),
            data,
            music_data,
            indexes: archives,
            legacy,
        }
    }

    fn check_archive(&self, archive: u8) {
        if archive > MAX_ARCHIVE as u8 {
            panic!("archive {} is out of bounds", archive);
        }
    }

    fn get_data(&self, archive: u8) -> &Mmap {
        if archive == MUSIC_ARCHIVE && self.music_data.is_some() {
            self.music_data.as_ref().unwrap()
        } else {
            &self.data
        }
    }

    fn check_group(&self, archive: u8, group: u16) {
        self.check_archive(archive);

        if group < 0 {
            panic!("group {} is out of bounds", group);
        }
    }

    fn read_index_entry(&self, archive: u8, group: u16) -> Option<IndexEntry> {
        self.check_group(archive, group);

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

struct ArchiveOld {
    dirty: bool,
}

trait Archive {
    fn is_dirty(&self) -> bool;
    fn read(&self, group: u16, file: u16, xtea_keys: Option<[u32; 4]>) -> Vec<u8>;
    fn get_unpacked(&self, entry: &Js5IndexEntry, key: Option<[u32; 4]>) -> Unpacked;
    fn read_packed(&self, group: u32) -> Vec<u8>;
    fn verify_compressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry);
    fn verify_uncompressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry);
}

struct Group {}

impl Group {
    pub fn unpack(buf: Vec<u8>, entry: &Js5IndexEntry) {}
}

const MAX_ARCHIVE: usize = 255;
const MAX_GROUP_SIZE: usize = (1 << 24) - 1;
const ARCHIVESET: usize = (1 << 24) - 1;

trait Store {
    fn list(&self, archive: u8) -> Vec<u32>;
    fn read(&self, archive: u8, group: u16) -> Vec<u8>;
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
}

impl CacheArchive {
    pub fn testy() {}
}

struct Unpacked {
    dirty: bool,
}

impl Unpacked {
    pub fn read(file: u32) -> Vec<u8> {
        Vec::new()
    }
}

impl Archive for CacheArchive {
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn read(&self, group: u16, file: u16, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        if group < 0 || file < 0 {
            panic!("group {} or file {} is out of bounds", group, file);
        }

        let entry = self.index.groups.get(&(group as u32)).unwrap();

        Vec::new()
    }

    fn get_unpacked(&self, entry: &Js5IndexEntry, key: Option<[u32; 4]>) -> Unpacked {
        // TODO: Handle unpacked cache

        let compressed = self.read_packed(123);

        self.verify_compressed(&compressed, entry);

        let buf = Js5Compression::uncompress(compressed, key);

        self.verify_uncompressed(&buf, entry);

        Unpacked { dirty: false }
    }

    fn read_packed(&self, group: u32) -> Vec<u8> {
        Vec::new()
    }

    fn verify_compressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry) {}

    fn verify_uncompressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry) {}
}

impl ArchiveOld {
    pub fn read(&self, group: u16, file: u16, data: &Mmap) -> Vec<u8> {
        Vec::new()
    }
}

pub struct Cache {
    /// Store
    store_new: Box<dyn Store>,

    /// Archives
    archives: HashMap<u8, CacheArchive>,

    /// Unpacked cache size
    unpacked_cache_size: usize,
}

enum Js5Protocol {
    Original = 5,
    Versioned = 6,
    Smart = 7,
}

enum Js5IndexFlags {
    FlagNames = 0x1,
    FlagDigests = 0x2,
    FlagLengths = 0x4,
    FlagUncompressedChecksums = 0x8,
}

#[derive(Debug)]
struct Js5IndexFile {
    name_hash: i32,
}

#[derive(Debug)]
struct Js5IndexEntry {
    name_hash: i32,
    version: u32,
    checksum: u32,
    uncompressed_checksum: u32,
    length: u32,
    uncompressed_length: u32,
    digest: Option<bool>,
    capacity: u32,
    files: HashMap<u32, Js5IndexFile>,
}

struct Js5Index {
    protocol: u8,
    version: i32,
    has_names: bool,
    has_digests: bool,
    has_lengths: bool,
    has_uncompressed_checksums: bool,
    groups: BTreeMap<u32, Js5IndexEntry>,
}

impl Js5Index {
    fn read<T: AsRef<[u8]>>(buf: T) -> Js5Index {
        trace!("Length of buffer: {}", buf.as_ref().len());

        let mut buf_ref = buf.as_ref();

        let protocol = buf_ref.read_u8().unwrap();

        let read_func = if protocol >= Js5Protocol::Smart as u8 {
            // TODO: Read Smart
            |v: &mut &[u8]| -> u32 { todo!() }
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

        // Trace flags and size
        trace!("JS5 Protocol: {}", protocol);
        trace!("JS5 Version: {}", version);
        trace!("JS5 Flags: {}", flags);
        trace!("JS5 Size: {}", size);

        // Create Js5Index
        let mut index = Js5Index {
            protocol,
            version,
            has_names: (flags & Js5IndexFlags::FlagNames as u8) != 0,
            has_digests: (flags & Js5IndexFlags::FlagDigests as u8) != 0,
            has_lengths: (flags & Js5IndexFlags::FlagLengths as u8) != 0,
            has_uncompressed_checksums: (flags & Js5IndexFlags::FlagUncompressedChecksums as u8)
                != 0,
            groups: BTreeMap::new(),
        };

        trace!("Creating groups");

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
                    digest: None,
                    capacity: 0,
                    files: HashMap::new(),
                },
            );
        });

        trace!("has_names: {}", index.has_names);

        if index.has_names {
            for (id, group) in &mut index.groups {
                group.name_hash = buf_ref.read_i32().unwrap();
            }
        }

        trace!("group checksum");

        for (id, group) in &mut index.groups {
            group.checksum = buf_ref.read_u32().unwrap();
        }

        trace!(
            "has_uncompressed_checksums: {}",
            index.has_uncompressed_checksums
        );

        if index.has_uncompressed_checksums {
            for (id, group) in &mut index.groups {
                group.uncompressed_checksum = buf_ref.read_u32().unwrap();
            }
        }

        trace!("has_digests: {}", index.has_digests);

        // TODO: Digests
        if index.has_digests {
            todo!("Digests");
            //for group in &mut index.entries {
            //}
        }

        trace!("has_lengths: {}", index.has_lengths);

        if index.has_lengths {
            for (id, group) in &mut index.groups {
                group.length = buf_ref.read_u32().unwrap();
                group.uncompressed_length = buf_ref.read_u32().unwrap();
            }
        }

        trace!("group_version");

        for (id, group) in &mut index.groups {
            group.version = buf_ref.read_u32().unwrap();
        }

        let group_sizes: Vec<u32> = (0..size).map(|_| read_func(&mut buf_ref)).collect();

        trace!("group_sizes: {}", group_sizes.len());

        trace!("Trace1, size: {}", buf_ref.len());

        for (i, (id, group)) in index.groups.iter_mut().enumerate() {
            let group_size = group_sizes[i];

            let mut prev_file_id = 0;
            (0..group_size).for_each(|_| {
                prev_file_id += read_func(&mut buf_ref);
                group
                    .files
                    .insert(prev_file_id, Js5IndexFile { name_hash: -1 });
            });
        }

        trace!("Trace9");

        if index.has_names {
            for (id, group) in &mut index.groups {
                for (file_id, file) in &mut group.files {
                    file.name_hash = buf_ref.read_i32().unwrap();
                }
            }
        }

        trace!("Trace10");

        index
    }
}

const MAX_INDEXES: usize = 255;
const META_INDEX: usize = 255;
const INDEX_PATH: &str = "main_file_cache.idx";
const DATA_PATH: &str = "main_file_cache.dat2";
const LEGACY_DATA_PATH: &str = "main_file_cache.dat2";
const MUSIC_DATA_PATH: &str = "main_file_cache.dat2m";
const UNPACKED_CACHE_SIZE_DEFAULT: usize = 1024;

impl Cache {
    pub fn open(input_path: &str) -> io::Result<Cache> {
        let mut cache = Self {
            store_new: store_open(input_path),
            archives: HashMap::new(),
            unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
        };
        cache.init();

        // Return the Cache struct
        Ok(cache)
    }

    fn init(&mut self) {
        for archive in self.store_new.list(ARCHIVESET as u8) {
            //trace!("Loading archive {}", archive);
            let compressed = self.store_new.read(ARCHIVESET as u8, archive as u16);

            let buf = Js5Compression::uncompress(compressed, None);
            trace!("Uncompressed archive {} to {} bytes", archive, buf.len());

            let js5_index = Js5Index::read(buf);

            let cache_archive = CacheArchive {
                is_dirty: false,
                index: js5_index,
                archive: archive as u8,
            };

            self.archives.insert(archive as u8, cache_archive);
        }
    }

    /*pub fn open(input_path: &str) -> io::Result<Cache> {
        // Create a Path using the input path
        let cache_path = Path::new(input_path);

        // Create HashMap for storing the index files
        let mut indexes = HashMap::new();

        // Iterate over all indexes from 0 to including MAX_INDEXES (255)
        for i in 0..=MAX_INDEXES {
            let index_file = cache_path.join(format!("{}{}", INDEX_PATH, i));

            // If read from file, insert into HashMap
            if let Ok(index_file) = File::open(index_file.to_str().unwrap()) {
                if let Ok(index_file_mmap) = unsafe { Mmap::map(&index_file) } {
                    indexes.insert(i, index_file_mmap);
                }
            }
        }

        // Load the dat2 file
        let data_file_path = cache_path.join(DATA_PATH);
        let data_file =
            File::open(data_file_path.to_str().unwrap()).expect("failed getting data file");
        let data_file_mmap = unsafe { Mmap::map(&data_file)? };

        // Return the Cache struct
        Ok(Self {
            store: DiskStore {
                data: data_file_mmap,
                indexes,
                root: input_path.to_string(),
                music_data: None,
                legacy: false,
            },
            archives: HashMap::new(),
            unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
            store_new: store_open(input_path),
        })
    }*/

    /// Read a file from the cache
    ///
    /// # Arguments
    ///
    /// * `archive` - The archive to read from
    /// * `group` - The group to read from
    /// * `file` - The file to read
    /// * `xtea_keys` - The XTEA keys to use for decryption. If None, the file will not be decrypted
    pub fn read(
        &self,
        archive: u16,
        group: u16,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
    ) -> Vec<u8> {
        let archive_here = &self.archives[&(archive as u8)].read(group, file, xtea_keys);

        // Instructions on cache.read(2, 10, 1042):
        /*
        read the js5index in (255, 2) - though note this is cached in my cache lib so it doesn't need to re-read it every time
        find group 10 in the the js5index (SHOULD BE AT THIS STEP)
        find file 1042 inside group 10 in the js5index
        read group (2, 10) - note there's also a cache for this in my cache lib so it's faster if you need to read multiple files from the same group in succession
        read file 1042 from the group and return it

        // Group: https://git.openrs2.org/openrs2/openrs2/src/branch/master/cache/src/main/kotlin/org/openrs2/cache/Group.kt
        // Js5Index: https://git.openrs2.org/openrs2/openrs2/src/branch/master/cache/src/main/kotlin/org/openrs2/cache/Js5Index.kt
        */

        // Read (255,2)
        /*let archive_data = self.read_archive_group_data(META_INDEX, archive);

        trace!("Output len of (255,2) data: {}", archive_data.len());

        let mut csr = Cursor::new(&archive_data);

        // Find group 10
        let protocol = csr.read_u8().unwrap();

        let read_func = if protocol >= Js5Protocol::Smart as u8 {
            // TODO: Read Smart
            |v: &mut Cursor<&Vec<u8>>| -> u32 { todo!() }
        } else {
            |v: &mut Cursor<&Vec<u8>>| -> u32 { v.read_u16().unwrap() as u32 }
        };

        let version = if protocol >= Js5Protocol::Versioned as u8 {
            csr.read_i32().unwrap()
        } else {
            0
        };

        let flags = csr.read_u8().unwrap();
        let size = read_func(&mut csr);

        // Trace flags and size
        trace!("Flags: {}", flags);
        trace!("Size: {}", size);

        // Create Js5Index
        let mut index = Js5Index {
            protocol,
            version,
            has_names: (flags & Js5IndexFlags::FlagNames as u8) != 0,
            has_digests: (flags & Js5IndexFlags::FlagDigests as u8) != 0,
            has_lengths: (flags & Js5IndexFlags::FlagLengths as u8) != 0,
            has_uncompressed_checksums: (flags & Js5IndexFlags::FlagUncompressedChecksums as u8)
                != 0,
            groups: BTreeMap::new(),
        };

        // Begin creating the groups
        let mut prev_group_id = 0;
        (0..size).for_each(|_| {
            prev_group_id += read_func(&mut csr);
            index.groups.insert(
                prev_group_id,
                Js5IndexEntry {
                    name_hash: -1,
                    version: 0,
                    checksum: 0,
                    uncompressed_checksum: 0,
                    length: 0,
                    uncompressed_length: 0,
                    digest: None,
                    capacity: 0,
                    files: HashMap::new(),
                },
            );
        });

        if index.has_names {
            for (id, group) in &mut index.groups {
                group.name_hash = csr.read_i32().unwrap();
            }
        }

        for (id, group) in &mut index.groups {
            group.checksum = csr.read_u32().unwrap();
        }

        if index.has_uncompressed_checksums {
            for (id, group) in &mut index.groups {
                group.uncompressed_checksum = csr.read_u32().unwrap();
            }
        }

        // TODO: Digests
        if index.has_digests {
            //for group in &mut index.entries {
            //}
        }

        if index.has_lengths {
            for (id, group) in &mut index.groups {
                group.length = csr.read_u32().unwrap();
                group.uncompressed_length = csr.read_u32().unwrap();
            }
        }

        for (id, group) in &mut index.groups {
            group.version = csr.read_u32().unwrap();
        }

        let group_sizes: Vec<u32> = (0..size).map(|_| read_func(&mut csr)).collect();

        for (i, (id, group)) in index.groups.iter_mut().enumerate() {
            let group_size = group_sizes[i];

            let mut prev_file_id = 0;
            (0..group_size).for_each(|_| {
                prev_file_id += read_func(&mut csr);
                group
                    .files
                    .insert(prev_file_id, Js5IndexFile { name_hash: -1 });
            });
        }

        if index.has_names {
            for (id, group) in &mut index.groups {
                for (file_id, file) in &mut group.files {
                    file.name_hash = csr.read_i32().unwrap();
                }
            }
        }

        // Print data of the "items" group in the cache aka group 10
        /*trace!(
            "Len of files: {}",
            index.groups.get(&10).unwrap().files.len()
        );*/

        // EVERYTHING ABOVE FROM HERE SHOULD BE DONE ON CACHE OPENING, NOT IN READING

        // TODO: EVERYTHING BELOW SHOULD BE CACHED UPON FIRST READ

        // Read (2, 10)
        let archive_data2 = self.read_archive_group_data(archive as usize, group);
        trace!("Output size of compressed data: {}", archive_data2.len());

        trace!(
            "Some data here: {} {} {}",
            archive_data2[0],
            archive_data2[1],
            archive_data2[2]
        );

        // Now begin going over the stripes
        let stripes = *archive_data2.last().unwrap();
        trace!("Stripes: {}", stripes);

        let data_index = 0;
        let trailer_index = archive_data2.len()
            - (stripes as usize * index.groups.get(&(group as u32)).unwrap().files.len() * 4)
                as usize
            - 1;

        trace!("Trailer index: {}", trailer_index);

        let mut readerrr = Cursor::new(&archive_data2[trailer_index..]);

        let mut lens = vec![0; index.groups.get(&(group as u32)).unwrap().files.len()];

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in &mut lens {
                prev_len += readerrr.read_i32().unwrap();
                *j += prev_len;
            }
        }

        let mut file_reader_stuff = Cursor::new(&archive_data2);

        let mut files_final: BTreeMap<u32, Vec<u8>> = BTreeMap::new();

        for (x, y) in &index.groups.get(&(group as u32)).unwrap().files {
            files_final.insert(*x, vec![0; lens[*x as usize] as usize]);
        }

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in 0..index.groups.get(&(group as u32)).unwrap().files.len() {
                prev_len += lens[j];
                file_reader_stuff
                    .read_exact(&mut files_final.get_mut(&(j as u32)).unwrap())
                    .unwrap();
            }
        }

        files_final.get(&(file as u32)).unwrap().to_vec()*/
        Vec::new()
    }

    fn read_archive_group_data(&self, archive: usize, group: u16) -> Vec<u8> {
        let x = self.fun_name(archive, group);
        decompress_archive(x)
    }

    fn fun_name(&self, archive: usize, group: u16) -> Vec<u8> {
        // Get the archive (index file)
        /*let index_data = self
            .store
            .indexes
            .get(&archive)
            .unwrap_or_else(|| panic!("index file with id {} was not found", group));
        // Read archive header data
        let offset = group as usize * 6;
        let archive_len = u32::from_be_bytes([
            0,
            index_data[offset],
            index_data[offset + 1],
            index_data[offset + 2],
        ]);
        let archive_sector = u32::from_be_bytes([
            0,
            index_data[offset + 3],
            index_data[offset + 4],
            index_data[offset + 5],
        ]);
        let mut offset = archive_sector * 520;
        let mut sector = archive_sector;
        let mut read_bytes_count = 0;
        let mut part = 0;
        let mut archive_data = Vec::new();
        let mut temp_archive_buffer = [0u8; 520];
        while read_bytes_count < archive_len {
            // Sector should not be 0
            if sector == 0 {
                panic!("Sector 0");
            }

            // Get the block size
            let data_block_size = cmp::min(archive_len - read_bytes_count, 512);

            // Calulate the length of the data block
            let header_size = 8;
            let length = data_block_size + header_size;

            // Copy over new data to the temp buffer
            for i in 0..length {
                //println!("{} {}", i + offset, self.data[(i + offset) as usize]);
                temp_archive_buffer[i as usize] = self.store.data[(i + offset) as usize];
            }

            // Parse header values from the temp buffer
            let group_id = u16::from_be_bytes([temp_archive_buffer[0], temp_archive_buffer[1]]);
            let part_id = u16::from_be_bytes([temp_archive_buffer[2], temp_archive_buffer[3]]);
            let next_sector = u32::from_be_bytes([
                0,
                temp_archive_buffer[4],
                temp_archive_buffer[5],
                temp_archive_buffer[6],
            ]);

            // Test the header values against the expected values
            assert_eq!(group, group_id);
            assert_eq!(part, part_id);
            assert_ne!(sector, next_sector);

            // Add new data to archive data and update the read bytes count
            archive_data.extend(temp_archive_buffer[header_size as usize..length as usize].iter());
            read_bytes_count += length - header_size;

            // Get next sector
            sector = next_sector;

            // Increment part
            part += 1;

            // Update offset
            offset = sector * 520;
        }*/

        let archive_data = Vec::new();
        archive_data
    }
}

const COMPRESSION_TYPE_NONE: u8 = 0;
const COMPRESSION_TYPE_BZIP: u8 = 1;
const COMPRESSION_TYPE_GZIP: u8 = 2;

/*enum CompressionType {
    None = 0,
    Bzip = 1,
    Gzip = 2,
}*/

// Decompresses an archive file using the compression type specified in the header
fn decompress_archive(mut archive_data: Vec<u8>) -> Vec<u8> {
    let mut header_length = 1;

    trace!("Archive data len: {}", archive_data.len());

    // Get the type of compression used
    let compression_type = archive_data[0];
    trace!("Compression type: {}", compression_type);

    // Get compressed size
    let archive_size = u32::from_be_bytes([
        archive_data[1],
        archive_data[2],
        archive_data[3],
        archive_data[4],
    ]);
    trace!("Archive size: {}", archive_size);

    if compression_type == COMPRESSION_TYPE_NONE {
        header_length += 4;
    } else {
        header_length += 8;
    }

    // Remove the two version bytes if they exist
    if archive_data.len() == (archive_size + header_length + 2) as usize {
        archive_data.pop();
        archive_data.pop();
    };

    // If the compression type is none, return the data at this point
    if compression_type == COMPRESSION_TYPE_NONE {
        todo!("None compression not handled yet, check with OpenRS2 first")
        //return archive_data;
    }

    // Get the decompressed size
    let decompressed_size = u32::from_be_bytes([
        archive_data[5],
        archive_data[6],
        archive_data[7],
        archive_data[8],
    ]);
    trace!("Decompressed size: {}", decompressed_size);

    // Decompress the data based on the compression type
    let decompressed_data = match compression_type {
        COMPRESSION_TYPE_NONE => archive_data[9..].to_vec(),
        COMPRESSION_TYPE_BZIP => decompress_archive_bzip2(archive_data, decompressed_size),
        COMPRESSION_TYPE_GZIP => decompress_archive_gzip(archive_data, decompressed_size),
        _ => panic!("Unknown compression type: {}", compression_type),
    };

    decompressed_data
}

// Decompress using bzip2
fn decompress_archive_bzip2(archive_data: Vec<u8>, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut compressed_data = archive_data[5..archive_data.len() - 4].to_vec();
    compressed_data[..4].copy_from_slice(b"BZh1");
    let mut decompressor = BzDecoder::new(compressed_data.as_slice());

    decompressor.read_exact(&mut decompressed_data).unwrap();
    decompressed_data
}

// Decompress using gzip
fn decompress_archive_gzip(archive_data: Vec<u8>, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    // Skip the first 9 bytes of the archive data to get to the gzip header
    let mut decompressor = GzDecoder::new(&archive_data[9..]);
    decompressor.read_exact(&mut decompressed_data).unwrap();

    decompressed_data
}

#[no_mangle]
pub unsafe extern "C" fn cache_create(cache_ptr: *mut Cache, archive: u32) {}
#[no_mangle]
pub unsafe extern "C" fn cache_capacity(cache_ptr: *mut Cache, archive: u32) {}

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

#[no_mangle]
pub unsafe extern "C" fn cache_read(
    // Pointer to the cache
    cache_ptr: *mut Cache,
    // Archive id
    archive: u16,
    group: u16,
    file: u16,
    xtea_keys_arg: *const [u32; 4],
    // Output length
    out_len: *mut u32,
) -> *mut u8 {
    trace!("cache_read(cache_ptr = {:?}, archive = {}, group = {}, file = {}, xtea_keys = {:?}, out_len = {:?})", cache_ptr, archive, group, file, xtea_keys_arg, out_len);

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

#[no_mangle]
pub unsafe extern "C" fn free_cache_read_buffer(buffer: *mut u8) {
    // If the buffer is not null, drop the Vec
    if !buffer.is_null() {
        drop(Vec::from_raw_parts(buffer, 0, 0))
    }
}

#[no_mangle]
pub unsafe extern "C" fn cache_write(cache_ptr: *mut Cache, archive: u32) {}
#[no_mangle]
pub unsafe extern "C" fn cache_remove(cache_ptr: *mut Cache, archive: u32) {}

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
}
