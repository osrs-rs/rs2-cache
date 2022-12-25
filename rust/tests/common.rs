use osrscache::Cache;
use std::ffi::CString;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub fn setup() -> *mut Cache {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).ok();

    let cache = CString::new("./data/cache").unwrap();
    unsafe { osrscache::cache_open(cache.as_ptr()) }
}
