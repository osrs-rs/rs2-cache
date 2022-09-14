use bzip2::read::BzDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{self, Read},
    mem,
    os::raw::c_char,
    path::Path,
};
use thiserror::Error;

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
    data: Vec<u8>,

    /// Indexes
    indexes: Vec<Vec<u8>>,
}

const MAX_INDEXES: u8 = 255;
const META_INDEX: usize = 255;
static CACHE_INDEX_FILE_NAME: &str = "main_file_cache.idx";
static CACHE_DATA_FILE_NAME: &str = "main_file_cache.dat2";

impl Cache {
    pub fn open(input_path: &str) -> Cache {
        // Create a Path using the input path
        let cache_path = Path::new(input_path);

        // Create vector for storing the index files
        let mut indexes = Vec::new();

        // Iterate over all indexes
        for i in 0..=MAX_INDEXES {
            let index_file = cache_path.join(format!("{}{}", CACHE_INDEX_FILE_NAME, i));

            // Temp empty vec
            let mut index_file_data = Vec::new();

            // If read from file, set the new data
            if let Ok(read_index_file) = fs::read(index_file.to_str().unwrap()) {
                index_file_data = read_index_file;
            }

            // Add the index to the vec
            indexes.push(index_file_data);
        }

        // Load the dat2 file
        let data_file = cache_path.join(CACHE_DATA_FILE_NAME);
        let data = fs::read(data_file.to_str().unwrap()).expect("failed getting data file");

        Cache { data, indexes }
    }
    pub fn read(&self, archive: u16, group: u16, file: u16, xtea_keys: Option<[i32; 4]>) {
        // Instructions on cache.read(2, 10, 1042):
        /*
        read the js5index in (255, 2) - though note this is cached in my cache lib so it doesn't need to re-read it every time
        find group 10 in the the js5index
        find file 1042 inside group 10 in the js5index
        read group (2, 10) - note there's also a cache for this in my cache lib so it's faster if you need to read multiple files from the same group in succession
        read file 1042 from the group and return it

        // Group: https://git.openrs2.org/openrs2/openrs2/src/branch/master/cache/src/main/kotlin/org/openrs2/cache/Group.kt
        // Js5Index: https://git.openrs2.org/openrs2/openrs2/src/branch/master/cache/src/main/kotlin/org/openrs2/cache/Js5Index.kt
        */

        // Read (255,2), get compressed data back
        let mut archive_data = self.read_something(META_INDEX, archive);

        println!("Output size of compressed data: {}", archive_data.len());

        // Decompress (255,2)
        let decompressed_data = decompress_something_bzip(archive_data);

        println!(
            "Output size of decompressed data: {}",
            decompressed_data.len()
        );

        // Find group 10
        let protocol = decompressed_data[0];

        let smart = 0;
        let versioned = 0;

        let read_func = if protocol >= smart {
            |v: &Vec<u8>| -> u32 { v[2] as u32 }
        } else {
            |v: &Vec<u8>| -> u32 { v[3] as u32 }
        };

        let version = if protocol >= versioned {
            // read int
            12312321
        } else {
            0
        };
        let flags = 0;
        let size = read_func(&decompressed_data);

        // Write output to file
        fs::write("test.bin", decompressed_data).unwrap();
    }

    fn read_something(&self, archive: usize, group: u16) -> Vec<u8> {
        // Get the archive (index file)
        let index_data = self
            .indexes
            .get(archive)
            .expect(format!("index file with id {} was not found", group).as_str());

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
    // Get the type of compression used
    let compression_type = archive_data[0];
    // Get compressed size
    let compressed_size = u32::from_be_bytes([
        archive_data[1],
        archive_data[2],
        archive_data[3],
        archive_data[4],
    ]);
    println!("Compressed size check: {}", compressed_size);
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
    println!("Decompressed size check: {}", decompressed_size);

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
    len: *mut u32,
) -> *mut u8 {
    // Dereference the cache
    let cache = &*cache_ptr;

    // Dereference the xtea keys if not null
    let mut xtea_keys = None;
    if !xtea_keys_arg.is_null() {
        xtea_keys = Some(*xtea_keys_arg);
    }

    // Call the read function
    cache.read(archive, group, file, xtea_keys);

    // TODO: Return proper output
    let mut buf = vec![0; 512].into_boxed_slice();
    let data = buf.as_mut_ptr();
    *len = buf.len() as u32;
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
