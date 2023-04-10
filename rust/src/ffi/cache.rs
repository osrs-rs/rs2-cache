use crate::Cache;
use std::{ffi::CStr, mem, os::raw::c_char};

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
    let path_str = path_cstr.to_str().expect("failed to convert path to str");

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
    let cache = &mut *cache_ptr;

    // Dereference the xtea keys if not null
    let mut xtea_keys = None;
    if !xtea_keys_arg.is_null() {
        xtea_keys = Some(*xtea_keys_arg);
    }

    // Call the read function
    let mut buf = cache
        .read(archive, group, file, xtea_keys)
        .expect("failed reading file");

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
    let cache = &mut *cache_ptr;

    // Dereference the xtea keys if not null
    let mut xtea_keys = None;
    if !xtea_keys_arg.is_null() {
        xtea_keys = Some(*xtea_keys_arg);
    }

    let group_str = CStr::from_ptr(group)
        .to_str()
        .expect("failed to convert group to str");

    // Call the read function
    let mut buf = cache
        .read_named_group(archive, group_str, file, xtea_keys)
        .expect("failed reading named group");

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
///
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
///
/// - The caller must ensure that the pointer is valid.
/// - The caller should also ensure that the cache is not used after it has been closed.
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
}
