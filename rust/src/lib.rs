use bzip2::read::BzDecoder;
use memmap2::Mmap;
use osrs_bytes::ReadExt;
use std::{
    cmp,
    collections::{BTreeMap, HashMap},
    ffi::CStr,
    fs::{self, File},
    io::{self, Cursor, Read},
    mem,
    os::raw::c_char,
    path::Path,
};
use thiserror::Error;
use tracing::trace;

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

struct DiskStore {
    data: Mmap,

    indexes: HashMap<usize, Mmap>,
}

struct Archive {
    dirty: bool,
}

impl Archive {
    pub fn read(&self, group: u16, file: u16, data: &Mmap) -> Vec<u8> {
        Vec::new()
    }
}

pub struct Cache {
    /// Store
    store: DiskStore,

    /// Archives
    archives: HashMap<u16, Archive>,
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

const MAX_INDEXES: usize = 255;
const META_INDEX: usize = 255;
static CACHE_INDEX_FILE_NAME: &str = "main_file_cache.idx";
static CACHE_DATA_FILE_NAME: &str = "main_file_cache.dat2";

impl Cache {
    pub fn open(input_path: &str) -> io::Result<Cache> {
        // Create a Path using the input path
        let cache_path = Path::new(input_path);

        // Create HashMap for storing the index files
        let mut indexes = HashMap::new();

        // Iterate over all indexes from 0 to including MAX_INDEXES (255)
        for i in 0..=MAX_INDEXES {
            let index_file = cache_path.join(format!("{}{}", CACHE_INDEX_FILE_NAME, i));

            // If read from file, insert into HashMap
            if let Ok(index_file) = File::open(index_file.to_str().unwrap()) {
                if let Ok(index_file_mmap) = unsafe { Mmap::map(&index_file) } {
                    indexes.insert(i, index_file_mmap);
                }
            }
        }

        // Load the dat2 file
        let data_file_path = cache_path.join(CACHE_DATA_FILE_NAME);
        let data_file =
            File::open(data_file_path.to_str().unwrap()).expect("failed getting data file");
        let data_file_mmap = unsafe { Mmap::map(&data_file)? };

        // Return the Cache struct
        Ok(Self {
            store: DiskStore {
                data: data_file_mmap,
                indexes,
            },
            archives: HashMap::new(),
        })
    }

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
        xtea_keys: Option<[i32; 4]>,
    ) -> Vec<u8> {
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
        let archive_data = self.read_archive_group_data(META_INDEX, archive);

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
        trace!(
            "Len of files: {}",
            index.groups.get(&10).unwrap().files.len()
        );

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
            - (stripes as usize * index.groups.get(&10).unwrap().files.len() * 4) as usize
            - 1;

        trace!("Trailer index: {}", trailer_index);

        let mut readerrr = Cursor::new(&archive_data2[trailer_index..]);

        let mut lens = vec![0; index.groups.get(&10).unwrap().files.len()];

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in &mut lens {
                prev_len += readerrr.read_i32().unwrap();
                *j += prev_len;
            }
        }

        let mut file_reader_stuff = Cursor::new(&archive_data2);

        let mut files_final: BTreeMap<u32, Vec<u8>> = BTreeMap::new();

        for (x, y) in &index.groups.get(&10).unwrap().files {
            files_final.insert(*x, vec![0; lens[*x as usize] as usize]);
        }

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in 0..index.groups.get(&10).unwrap().files.len() {
                prev_len += lens[j];
                file_reader_stuff
                    .read_exact(&mut files_final.get_mut(&(j as u32)).unwrap())
                    .unwrap();
            }
        }

        files_final.get(&(file as u32)).unwrap().to_vec()
    }

    fn read_archive_group_data(&self, archive: usize, group: u16) -> Vec<u8> {
        let x = self.fun_name(archive, group);
        let y = decompress_archive(x);

        y
    }

    fn fun_name(&self, archive: usize, group: u16) -> Vec<u8> {
        // Get the archive (index file)
        let index_data = self
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
        }
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
        COMPRESSION_TYPE_BZIP => decompress_archive_bzip2(archive_data, decompressed_size),
        COMPRESSION_TYPE_GZIP => todo!("GZIP compression not implemented yet"),
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
    xtea_keys_arg: *const [i32; 4],
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
