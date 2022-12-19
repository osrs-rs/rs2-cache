use osrscache::Cache;
use std::ffi::CString;

pub fn setup() -> *mut Cache {
    let cache = CString::new("./data/cache").unwrap();
    unsafe { osrscache::cache_open(cache.as_ptr()) }
}
