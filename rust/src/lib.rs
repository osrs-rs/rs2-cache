use bzip2::read::BzDecoder;
use memmap2::Mmap;
use osrs_buffer::ReadExt;
use std::{
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

pub struct Cache {
    /// The data file
    data: Mmap,

    /// Indexes
    indexes: HashMap<usize, Mmap>,
}

enum Js5Protocol {
    Original = 5,
    Versioned = 6,
    Smart = 7,
}

enum Js5IndexFlags {
    Flag_Names = 0x1,
    Flag_Digests = 0x2,
    Flag_Lengths = 0x4,
    Flag_UncompressedChecksums = 0x8,
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
    entries: HashMap<u32, Js5IndexFile>,
}

struct Js5Index {
    protocol: u8,
    version: i32,
    has_names: bool,
    has_digests: bool,
    has_lengths: bool,
    has_uncompressed_checksums: bool,
    entries: BTreeMap<u32, Js5IndexEntry>,
}

const MAX_INDEXES: usize = 255;
const META_INDEX: usize = 255;
static CACHE_INDEX_FILE_NAME: &str = "main_file_cache.idx";
static CACHE_DATA_FILE_NAME: &str = "main_file_cache.dat2";

impl Cache {
    pub fn open(input_path: &str) -> Cache {
        // Create a Path using the input path
        let cache_path = Path::new(input_path);

        // Create HashMap for storing the index files
        let mut indexes = HashMap::new();

        // Iterate over all indexes
        for i in 0..=MAX_INDEXES {
            let index_file = cache_path.join(format!("{}{}", CACHE_INDEX_FILE_NAME, i));

            // If read from file, set the new data
            if let Ok(index_file) = File::open(index_file.to_str().unwrap()) {
                let index_file_mmap = unsafe { Mmap::map(&index_file).unwrap() };

                indexes.insert(i, index_file_mmap);
            }
        }

        // Load the dat2 file
        let data_file_path = cache_path.join(CACHE_DATA_FILE_NAME);
        let data_file =
            File::open(data_file_path.to_str().unwrap()).expect("failed getting data file");
        let data_file_mmap = unsafe { Mmap::map(&data_file).unwrap() };

        Self {
            data: data_file_mmap,
            indexes,
        }
    }
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

        // Read (255,2), get compressed data back
        let archive_data = self.read_something(META_INDEX, archive);

        trace!("Output size of compressed data: {}", archive_data.len());

        // Decompress (255,2)
        let decompressed_data = decompress_something_bzip(archive_data);

        trace!(
            "Output size of decompressed data: {}",
            decompressed_data.len()
        );

        let mut csr = Cursor::new(&decompressed_data);

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
            has_names: (flags & Js5IndexFlags::Flag_Names as u8) != 0,
            has_digests: (flags & Js5IndexFlags::Flag_Digests as u8) != 0,
            has_lengths: (flags & Js5IndexFlags::Flag_Lengths as u8) != 0,
            has_uncompressed_checksums: (flags & Js5IndexFlags::Flag_UncompressedChecksums as u8)
                != 0,
            entries: BTreeMap::new(),
        };

        // Begin creating the groups
        let mut prev_group_id = 0;
        (0..size).for_each(|_| {
            prev_group_id += read_func(&mut csr);
            index.entries.insert(
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
                    entries: HashMap::new(),
                },
            );
        });

        if index.has_names {
            for (id, group) in &mut index.entries {
                group.name_hash = csr.read_i32().unwrap();
            }
        }

        for (id, group) in &mut index.entries {
            group.checksum = csr.read_u32().unwrap();
        }

        if index.has_uncompressed_checksums {
            for (id, group) in &mut index.entries {
                group.uncompressed_checksum = csr.read_u32().unwrap();
            }
        }

        // TODO: Digests
        if index.has_digests {
            //for group in &mut index.entries {
            //}
        }

        if index.has_lengths {
            for (id, group) in &mut index.entries {
                group.length = csr.read_u32().unwrap();
                group.uncompressed_length = csr.read_u32().unwrap();
            }
        }

        for (id, group) in &mut index.entries {
            group.version = csr.read_u32().unwrap();
        }

        let group_sizes: Vec<u32> = (0..size).map(|_| read_func(&mut csr)).collect();

        for (i, (id, group)) in index.entries.iter_mut().enumerate() {
            let group_size = group_sizes[i];

            let mut prev_file_id = 0;
            (0..group_size).for_each(|_| {
                prev_file_id += read_func(&mut csr);
                group
                    .entries
                    .insert(prev_file_id, Js5IndexFile { name_hash: -1 });
            });
        }

        if index.has_names {
            for (id, group) in &mut index.entries {
                for (file_id, file) in &mut group.entries {
                    file.name_hash = csr.read_i32().unwrap();
                }
            }
        }

        // Print data of the "items" group in the cache aka group 10
        trace!(
            "Len of files: {}",
            index.entries.get(&10).unwrap().entries.len()
        );

        // EVERYTHING ABOVE FROM HERE SHOULD BE DONE ON CACHE OPENING, NOT IN READING

        // TODO: EVERYTHING BELOW SHOULD BE CACHED UPON FIRST READ

        // Grab 2,10. Decompress it, and begin going over the stripes
        let archive_data2 = self.read_something(archive as usize, group);
        trace!("Output size of compressed data: {}", archive_data2.len());

        // Decompress it
        let archive_data2_decompressed = decompress_something_bzip(archive_data2);

        trace!(
            "Some data here: {} {} {}",
            archive_data2_decompressed[0],
            archive_data2_decompressed[1],
            archive_data2_decompressed[2]
        );

        // Now begin going over the stripes
        let stripes = *archive_data2_decompressed.last().unwrap();
        trace!("Stripes: {}", stripes);

        let data_index = 0;
        let trailer_index = archive_data2_decompressed.len()
            - (stripes as usize * index.entries.get(&10).unwrap().entries.len() * 4) as usize
            - 1;

        trace!("Trailer index: {}", trailer_index);

        let mut readerrr = Cursor::new(&archive_data2_decompressed[trailer_index..]);

        let mut lens = vec![0; index.entries.get(&10).unwrap().entries.len()];

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in &mut lens {
                prev_len += readerrr.read_i32().unwrap();
                *j += prev_len;
            }
        }

        let mut file_reader_stuff = Cursor::new(&archive_data2_decompressed);

        let mut files_final: BTreeMap<u32, Vec<u8>> = BTreeMap::new();

        for (x, y) in &index.entries.get(&10).unwrap().entries {
            files_final.insert(*x, vec![0; lens[*x as usize] as usize]);
        }

        for i in 0..stripes {
            let mut prev_len = 0;
            for j in 0..index.entries.get(&10).unwrap().entries.len() {
                prev_len += lens[j];
                file_reader_stuff
                    .read_exact(&mut files_final.get_mut(&(j as u32)).unwrap())
                    .unwrap();
            }
        }

        files_final.get(&(file as u32)).unwrap().to_vec()
    }

    fn read_something(&self, archive: usize, group: u16) -> Vec<u8> {
        // Get the archive (index file)
        let index_data = self
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

            // Handle block size
            let mut data_block_size = archive_len - read_bytes_count;
            if data_block_size > 512 {
                data_block_size = 512;
            }

            // Calc new len
            let header_size = 8;
            let length = data_block_size + header_size;

            // Copy over new data
            for i in 0..length {
                //println!("{} {}", i + offset, self.data[(i + offset) as usize]);
                temp_archive_buffer[i as usize] = self.data[(i + offset) as usize];
            }

            // Parse header values
            let group_id = u16::from_be_bytes([temp_archive_buffer[0], temp_archive_buffer[1]]);
            let part_id = u16::from_be_bytes([temp_archive_buffer[2], temp_archive_buffer[3]]);
            let next_sector = u32::from_be_bytes([
                0,
                temp_archive_buffer[4],
                temp_archive_buffer[5],
                temp_archive_buffer[6],
            ]);

            // TODO: Verify here if everything is ok using the grabbed header

            for i in header_size..length {
                archive_data.push(temp_archive_buffer[i as usize]);
                read_bytes_count += 1;
            }

            // Get next sector
            sector = next_sector;
            part += 1;

            offset = sector * 520;
        }
        archive_data
    }
}

fn decompress_something_bzip(mut archive_data: Vec<u8>) -> Vec<u8> {
    trace!("Archive data len: {}", archive_data.len());

    // Get the type of compression used
    let compression_type = archive_data[0];
    trace!("Compression type: {}", compression_type);

    // Get compressed size
    let compressed_size = u32::from_be_bytes([
        archive_data[1],
        archive_data[2],
        archive_data[3],
        archive_data[4],
    ]);
    trace!("Compressed size: {}", compressed_size,);

    // Get decompressed size
    let mut decompressed_size = 0;
    if compression_type != 0 {
        decompressed_size = u32::from_be_bytes([
            archive_data[5],
            archive_data[6],
            archive_data[7],
            archive_data[8],
        ]);
    }
    trace!("Decompressed size: {}", decompressed_size);

    // Remove the version (2 bytes) TODO: Check if size needs removal, don't just plainly remove it
    archive_data.pop();
    archive_data.pop();
    // Decompress using bzip2 (only impl for now)
    // Copy over the compressed data, skipping 4 bytes for bzip header
    let mut compressed_data = archive_data[5..archive_data.len() - 4].to_vec();
    // Copy over the bzip header
    compressed_data[..4].copy_from_slice(b"BZh1");
    let mut decompressor = BzDecoder::new(compressed_data.as_slice());
    let mut decompressed_data = vec![0; decompressed_size as usize];
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
    let cache = Cache::open(path_str);

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
