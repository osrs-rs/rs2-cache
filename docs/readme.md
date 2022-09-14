# osrs-cache

[![Build](https://github.com/runecore/osrs-cache/workflows/build/badge.svg)](https://github.com/runecore/osrs-cache)
[![API](https://docs.rs/osrs-cache/badge.svg)](https://docs.rs/osrs-cache)
[![Crate](https://img.shields.io/crates/v/osrs-cache)](https://crates.io/crates/osrs-cache)
[![dependency status](https://deps.rs/repo/github/runecore/osrs-cache/status.svg)](https://deps.rs/repo/github/runecore/osrs-cache)
[![OSRS Version](https://img.shields.io/badge/OSRS-208-blue)](https://img.shields.io/badge/OSRS-208-blue)
[![Discord](https://img.shields.io/discord/926860365873184768?color=5865F2)](https://discord.gg/CcTa7TZfSc)

A read-only, high-level, virtual file API for the RuneScape cache.

## Installation

Add this to your `Cargo.toml` file:

```toml
[dependencies]
osrs-cache = "0.3.0"
```

## Example

```rust
use osrscache::Cache;

fn main() -> Result<(), osrscache::Error> {
    let cache = Cache::new("./data/osrs_cache")?;

    let index_id = 2; // Config index.
    let archive_id = 10; // Archive containing item definitions.

    let buffer = cache.read(index_id, archive_id)?;

    Ok(())
}
```

## Contributing

If you have suggestions for features, or want to add for example a new loader for the cache, feel free to make a pull request. For bigger features it is advised to [open an issue](https://github.com/runecore/osrs-cache/issues/new) in order to discuss it beforehand.

Examples can be found in the [examples](examples/) directory which include the osrs update protocol.
