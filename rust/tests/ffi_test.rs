use rs2cache::ffi::{cache_open, cache_read, cache_read_named_group};
use std::{ffi::CString, ptr, slice};

#[test]
fn test_cache_open() {
    let cache_str = CString::new("tests/data/cache/cache-read").unwrap();
    let cache_ptr = unsafe { cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());
}

#[test]
fn test_cache_read() {
    let cache_str = CString::new("tests/data/cache/cache-read").unwrap();
    let cache_ptr = unsafe { cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let buf = unsafe { cache_read(cache_ptr, 0, 0, 0, ptr::null(), out_len_ptr) };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!("OpenRS2".as_bytes(), buf_data)
}

#[test]
fn test_cache_read_encrypted() {
    let cache_str = CString::new("tests/data/cache/cache-read-encrypted").unwrap();
    let cache_ptr = unsafe { cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

    let mut out_len = 0;
    let out_len_ptr: *mut u32 = &mut out_len;

    let buf = unsafe { cache_read(cache_ptr, 0, 0, 0, &KEY as *const [u32; 4], out_len_ptr) };

    let buf_data = unsafe { slice::from_raw_parts(buf, out_len as usize) };

    assert_eq!("OpenRS2".as_bytes(), buf_data)
}

#[test]
fn test_cache_read_named_group() {
    let cache_str = CString::new("tests/data/cache/cache-read-named-group").unwrap();
    let cache_ptr = unsafe { cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

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

    assert_eq!("OpenRS2".as_bytes(), buf_data)
}

#[test]
fn test_cache_read_named_group_encrypted() {
    let cache_str = CString::new("tests/data/cache/cache-read-named-group-encrypted").unwrap();
    let cache_ptr = unsafe { cache_open(cache_str.as_ptr()) };
    assert!(!cache_ptr.is_null());

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

    assert_eq!("OpenRS2".as_bytes(), buf_data)
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
