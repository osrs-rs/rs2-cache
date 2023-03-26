pub use rs2cache::cache_read;
use rs2cache::{cache_read_named_group, Cache};
use std::{ffi::CString, ptr, slice};
use tracing::trace;

mod common;

#[test]
fn test_cache_read_2() {
    let cache = Cache::open("tests/data/cache/cache-read").unwrap();
    let buf = cache.read(0, 0, 0, None);
    assert_eq!(buf, [0x4f, 0x70, 0x65, 0x6e, 0x52, 0x53, 0x32])
}

// TODO: Make a test with gzip compressed data. I think the version thing may be fucked up on there in Js5Compression

/*#[test]
fn test_huffman() {
    let cache = Cache::open("data/cache").unwrap();

    let buf = cache.read_named(10, "huffman", 0, None);
    assert_eq!(
        buf,
        [
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x15, 0x16, 0x16, 0x14, 0x16, 0x16, 0x16, 0x15,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x03, 0x08, 0x16, 0x10, 0x16, 0x10, 0x11, 0x07, 0x0d, 0x0d,
            0x0d, 0x10, 0x07, 0x0a, 0x06, 0x10, 0x0a, 0x0b, 0x0c, 0x0c, 0x0c, 0x0c, 0x0d, 0x0d,
            0x0e, 0x0e, 0x0b, 0x0e, 0x13, 0x0f, 0x11, 0x08, 0x0b, 0x09, 0x0a, 0x0a, 0x0a, 0x0a,
            0x0b, 0x0a, 0x09, 0x07, 0x0c, 0x0b, 0x0a, 0x0a, 0x09, 0x0a, 0x0a, 0x0c, 0x0a, 0x09,
            0x08, 0x0c, 0x0c, 0x09, 0x0e, 0x08, 0x0c, 0x11, 0x10, 0x11, 0x16, 0x0d, 0x15, 0x04,
            0x07, 0x06, 0x05, 0x03, 0x06, 0x06, 0x05, 0x04, 0x0a, 0x07, 0x05, 0x06, 0x04, 0x04,
            0x06, 0x0a, 0x05, 0x04, 0x04, 0x05, 0x07, 0x06, 0x0a, 0x06, 0x0a, 0x16, 0x13, 0x16,
            0x0e, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x15, 0x16, 0x15, 0x16, 0x16,
            0x16, 0x15, 0x16, 0x16
        ]
    )
}*/

#[test]
fn test_cache_open() {
    let cache_str = CString::new("tests/data/cache/cache-read").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());
}

#[test]
fn test_cache_read() {
    let cache_str = CString::new("tests/data/cache/cache-read").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let buf = unsafe { cache_read(cache_ptr, 0, 0, 0, ptr::null(), out_len_ptr) };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!(buf_data, "OpenRS2".as_bytes())
}

#[test]
fn test_cache_read_encrypted() {
    let cache_str = CString::new("tests/data/cache/cache-read-encrypted").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let buf = unsafe { cache_read(cache_ptr, 0, 0, 0, &KEY as *const [u32; 4], out_len_ptr) };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!(buf_data, "OpenRS2".as_bytes())
}

#[test]
fn test_cache_read_named_group() {
    let cache_str = CString::new("tests/data/cache/cache-read-named-group").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let group_str = CString::new("OpenRS2").unwrap();

    let buf = unsafe {
        cache_read_named_group(
            cache_ptr,
            0,
            group_str.as_ptr(),
            0,
            ptr::null(),
            out_len_ptr,
        )
    };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!(buf_data, "OpenRS2".as_bytes())
}

#[test]
fn test_cache_read_named_group_encrypted() {
    let cache_str = CString::new("tests/data/cache/cache-read-named-group-encrypted").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let group_str = CString::new("OpenRS2").unwrap();

    let buf = unsafe {
        cache_read_named_group(
            cache_ptr,
            0,
            group_str.as_ptr(),
            0,
            &KEY as *const [u32; 4],
            out_len_ptr,
        )
    };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!(buf_data, "OpenRS2".as_bytes())
}

const KEY: [u32; 4] = [0x00112233, 0x44556677, 0x8899AABB, 0xCCDDEEFF];

/*#[test]
fn test_cache_read_bzip2() {
    let cache_str = CString::new("tests/data/cache/cache-read-bzip2").unwrap();
    let cache_ptr = unsafe { rs2cache::cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let buf = unsafe { cache_read(cache_ptr, 0, 0, 0, ptr::null(), out_len_ptr) };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!(buf_data, [0x4f, 0x70, 0x65, 0x6e, 0x52, 0x53, 0x32])
}*/
