# rs2-cache

[![Build](https://github.com/osrs-rs/rs2-cache/workflows/build/badge.svg)](https://github.com/osrs-rs/rs2-cache)
[![API](https://docs.rs/rs2-cache/badge.svg)](https://docs.rs/rs2-cache)
[![Crate](https://img.shields.io/crates/v/rs2-cache)](https://crates.io/crates/rs2-cache)
[![Discord](https://img.shields.io/discord/926860365873184768?color=5865F2)](https://discord.gg/CcTa7TZfSc)

A RS2 cache library written in Rust, based on the [OpenRS2](https://github.com/openrs2/openrs2) implementation.

It should be noted: RS2 also includes Old School Runescape.

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
rs2-cache = "0.2.0"
```

## Example

```rust
use rs2cache::Cache;

fn main() -> Result<(), rs2cache::cache::CacheError> {
    let cache = Cache::open("./cache")?;

    let index_id = 2; // Config index
    let archive_id = 10; // Item definitions archive
    let file_id = 1042; // Blue Partyhat file

    let buffer = cache.read(index_id, archive_id, file_id)?;

    Ok(())
}
```

## Contributing

If you have new features you would like to have implemented, feel free to open a pull request. Do be advised that this repository aims to follow OpenRS2, so any edits should not deviate too far from it (if at all). For bigger features it is advised to [open an issue](https://github.com/osrs-rs/rs2-cache/issues/new) in order to discuss it beforehand.

## Credits

- Graham: For his work on [OpenRS2](https://github.com/openrs2/openrs2). If you are using Kotlin and working on a RS2 rev server, it is highly recommended to utilise it for any cache related tasks.
- JayArrowz: Created the implementation that allows C# to use rs2-cache.