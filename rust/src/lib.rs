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
            COMPRESSION_TYPE_BZIP => decompress_archive_bzip2(plain_text, uncompressed_len as u32),
            COMPRESSION_TYPE_GZIP => decompress_archive_gzip(plain_text, uncompressed_len as u32),
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

    // Skip the first 9 bytes of the archive data to get to the gzip header
    let mut decompressor = GzDecoder::new(archive_data.as_ref());
    decompressor.read_exact(&mut decompressed_data).unwrap();

    decompressed_data
}

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

        /*if group < 0 {
            panic!("group {} is out of bounds", group);
        }*/
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

trait Archive {
    fn is_dirty(&self) -> bool;
    fn read(
        &self,
        group: u16,
        file: u16,
        xtea_keys: Option<[u32; 4]>,
        store: &Box<dyn Store>,
    ) -> Vec<u8>;
    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u16,
        key: Option<[u32; 4]>,
        store: &Box<dyn Store>,
    ) -> Unpacked;
    fn read_packed(&self, group: u16, store: &Box<dyn Store>) -> Vec<u8>;
    fn verify_compressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry);
    fn verify_uncompressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry);
}

struct Group {}

impl Group {
    pub fn unpack(
        buf: Vec<u8>,
        entry: &Js5IndexEntry,
        entry_id: u16,
        js5_index: &Js5Index,
    ) -> BTreeMap<u32, Vec<u8>> {
        // Now begin going over the stripes
        let stripes = *buf.last().unwrap();
        trace!("Stripes: {}", stripes);

        let data_index = 0;
        let trailer_index = buf.len()
            - (stripes as usize
                * js5_index
                    .groups
                    .get(&(entry_id as u32))
                    .unwrap()
                    .files
                    .len()
                * 4) as usize
            - 1;

        trace!("Trailer index: {}", trailer_index);

        let mut readerrr = Cursor::new(&buf[trailer_index..]);

        let mut lens = vec![
            0;
            js5_index
                .groups
                .get(&(entry_id as u32))
                .unwrap()
                .files
                .len()
        ];

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in &mut lens {
                prev_len += readerrr.read_i32().unwrap();
                *j += prev_len;
            }
        }

        let mut file_reader_stuff = Cursor::new(&buf);

        let mut files_final = BTreeMap::new();

        for (x, y) in &js5_index.groups.get(&(entry_id as u32)).unwrap().files {
            files_final.insert(*x, vec![0; lens[*x as usize] as usize]);
        }

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in 0..js5_index
                .groups
                .get(&(entry_id as u32))
                .unwrap()
                .files
                .len()
            {
                prev_len += lens[j];
                file_reader_stuff
                    .read_exact(&mut files_final.get_mut(&(j as u32)).unwrap())
                    .unwrap();
            }
        }

        files_final
    }
}

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
        self.files.get(&(file as u32)).unwrap().to_vec()
    }
}

impl Archive for CacheArchive {
    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn read(
        &self,
        group: u16,
        file: u16,
        key: Option<[u32; 4]>,
        store: &Box<dyn Store>,
    ) -> Vec<u8> {
        /*if group < 0 || file < 0 {
            panic!("group {} or file {} is out of bounds", group, file);
        }*/

        let entry = self.index.groups.get(&(group as u32)).unwrap();
        let unpacked = self.get_unpacked(entry, group, key, store);
        unpacked.read(file as u32)
    }

    fn get_unpacked(
        &self,
        entry: &Js5IndexEntry,
        entry_id: u16,
        key: Option<[u32; 4]>,
        store: &Box<dyn Store>,
    ) -> Unpacked {
        trace!("get unpacked");
        // TODO: Handle unpacked cache

        // DONE
        let compressed = self.read_packed(entry_id, &store);

        // DONE
        self.verify_compressed(&compressed, entry);

        // DONE
        let buf = Js5Compression::uncompress(compressed, key);

        // DONE
        self.verify_uncompressed(&buf, entry);

        // TODO
        let files = Group::unpack(buf, entry, entry_id, &self.index);

        let unpacked = Unpacked {
            dirty: false,
            key,
            files,
        };
        //self.unpacked_cache.insert(123, unpacked);

        unpacked
    }

    fn read_packed(&self, group: u16, store: &Box<dyn Store>) -> Vec<u8> {
        store.read(self.archive, group)
    }

    // TODO: Implement
    fn verify_compressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry) {}

    // TODO: Implement
    fn verify_uncompressed(&self, buf: &Vec<u8>, entry: &Js5IndexEntry) {}
}

pub struct Cache {
    /// Store
    store: Box<dyn Store>,

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

impl Cache {
    pub fn open(input_path: &str) -> io::Result<Cache> {
        let mut cache = Self {
            store: store_open(input_path),
            archives: HashMap::new(),
            unpacked_cache_size: UNPACKED_CACHE_SIZE_DEFAULT,
        };
        cache.init();

        // Return the Cache struct
        Ok(cache)
    }

    fn init(&mut self) {
        for archive in self.store.list(ARCHIVESET as u8) {
            //trace!("Loading archive {}", archive);
            let compressed = self.store.read(ARCHIVESET as u8, archive as u16);

            let buf = Js5Compression::uncompress(compressed, None);
            trace!("Uncompressed archive {} to {} bytes", archive, buf.len());

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
    pub fn read(&self, archive: u8, group: u16, file: u16, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        self.archives[&archive].read(group, file, xtea_keys, &self.store)
    }
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
    archive: u8,
    // Group id
    group: u16,
    // File id
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
