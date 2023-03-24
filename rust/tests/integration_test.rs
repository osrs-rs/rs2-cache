pub use rs2cache::cache_read;
use rs2cache::Cache;
use std::{ptr, slice};
use tracing::trace;

mod common;

#[test]
fn test_cache_open() {
    // Simply perform the setup as that is the same
    // Change this if the setup changes
    assert!(!common::setup().is_null())
}

#[test]
fn test_cache_read() {
    let cache = Cache::open("./data/cache").unwrap();

    cache.read(2, 10, 1042, None);
}

/*#[test]
fn test_cache_read() {
    // Open the cache
    let cache_ptr = common::setup();
    assert!(!cache_ptr.is_null());

    // Test reading blue partyhat (id 1042)
    // Create output length
    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    assert!(true);

    // Read from the cache
    /*let buf = unsafe { cache_read(cache_ptr, 2, 10, 1042, ptr::null(), out_len_ptr) };

    // Convert the output to a slice
    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    // Write out the output
    trace!("Output of buf_data: {:#04x?}", buf_data);

    assert_eq!(
        buf_data,
        [
            0x01, 0x0a, 0x4b, 0x07, 0x00, 0x01, 0x08, 0x00, 0x01, 0x04, 0x01, 0xb8, 0x06, 0x07,
            0x3c, 0x05, 0x00, 0x4c, 0x24, 0x57, 0x65, 0x61, 0x72, 0x00, 0x0d, 0x00, 0x17, 0x00,
            0xbb, 0x00, 0x19, 0x01, 0x6b, 0x00, 0x5a, 0x00, 0x1d, 0x5b, 0x00, 0x57, 0x4b, 0x00,
            0x38, 0x41, 0x28, 0x01, 0x03, 0x9e, 0xab, 0xc0, 0x02, 0x42, 0x6c, 0x75, 0x65, 0x20,
            0x70, 0x61, 0x72, 0x74, 0x79, 0x68, 0x61, 0x74, 0x00, 0x61, 0x04, 0x13, 0x94, 0x38,
            0x35, 0x00
        ]
    )*/
}*/
