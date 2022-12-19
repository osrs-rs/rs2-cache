use std::{ffi::CString, ptr, slice};

use osrscache::cache_read;

mod common;

#[test]
fn test_cache_open() {
    // Simply perform the setup as that is the same
    // Change this if the setup changes
    assert!(!common::setup().is_null())
}

#[test]
fn test_cache_read() {
    // Open the cache
    let cache_ptr = common::setup();

    // Test reading blue partyhat

    // Create output length
    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    // Read from the cache
    let buf = unsafe { cache_read(cache_ptr, 2, 10, 1042, ptr::null(), out_len_ptr) };
    //let buf = unsafe { cache_read(cache_ptr, 255, 2, 1042, out_len_ptr) };

    // Convert the output to a slice
    let v = unsafe { slice::from_raw_parts(buf, out_len as usize) };
    //println!("out_len: {}", out_len);
    //println!("out_len: {:?}", v);

    assert!(!cache_ptr.is_null())
}
