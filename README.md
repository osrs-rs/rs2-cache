# osrs-cache

[![Build](https://github.com/runecore/osrs-cache/workflows/build/badge.svg)](https://github.com/runecore/osrs-cache)
[![API](https://docs.rs/osrs-cache/badge.svg)](https://docs.rs/osrs-cache)
[![Crate](https://img.shields.io/crates/v/osrs-cache)](https://crates.io/crates/osrs-cache)
[![dependency status](https://deps.rs/repo/github/runecore/osrs-cache/status.svg)](https://deps.rs/repo/github/runecore/osrs-cache)
[![OSRS Version](https://img.shields.io/badge/OSRS-189-blue)](https://img.shields.io/badge/OSRS-189-blue)
[![Discord](https://img.shields.io/discord/926860365873184768?color=5865F2)](https://discord.gg/CcTa7TZfSc)

A read-only, high-level, virtual file API for the RuneScape cache.

This crate is based on the [rs-cache](https://github.com/jimvdl/rs-cache/) crate by jimvdl.

## Installation

Add this to your `Cargo.toml` file:

```toml
[dependencies]
osrs-cache = "0.1.0"
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

## Credits

The following sources aided with the development of this crate:

- [OpenRS](https://www.rune-server.ee/runescape-development/rs-503-client-server/downloads/312510-openrs-cache-library.html)
- [RuneLite](https://runelite.net/)
- [OSRS Cache Parsing Blog](https://www.osrsbox.com/blog/2018/07/26/osrs-cache-research-extract-cache-definitions/)
- [RSMod](https://github.com/Tomm0017/rsmod)
- [Librsfs](https://github.com/Velocity-/librsfs)
- [OSRSBox](https://www.osrsbox.com/)
- [Jagex-Store-5](https://github.com/guthix/Jagex-Store-5)

## License

`osrs-cache` is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
