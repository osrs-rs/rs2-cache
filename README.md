# osrs-cache

[![Build](https://github.com/runecore/osrs-cache/workflows/build/badge.svg)](https://github.com/runecore/osrs-cache)
[![API](https://docs.rs/osrs-cache/badge.svg)](https://docs.rs/osrs-cache)
[![Crate](https://img.shields.io/crates/v/osrs-cache)](https://crates.io/crates/osrs-cache)
[![dependency status](https://deps.rs/repo/github/runecore/osrs-cache/status.svg)](https://deps.rs/repo/github/runecore/osrs-cache)
[![OSRS Version](https://img.shields.io/badge/OSRS-189-blue)]()

An immutable, high-level API for the Oldschool RuneScape cache file system.

## Installation

Add this to your `Cargo.toml` file:

```toml
[dependencies]
osrs-cache = "0.1.0"
```

## Example

```rust
use osrscache::Cache;

fn main() -> osrscache::Result<()> {
    let cache = Cache::new("./data/cache")?;

    let index_id = 2; // Config index.
    let archive_id = 10; // Archive containing item definitions.

    let buffer: Vec<u8> = cache.read(index_id, archive_id)?;

    Ok(())
}
```

## Contributing

If you have suggestions for features, or want to add for example a new loader for the cache, feel free to make a pull request. For bigger features it is advised to [open an issue](https://github.com/runecore/osrs-cache/issues/new) in order to discuss it beforehand.

Examples can be found in the [examples](examples/) directory which include the osrs update protocol.

## Acknowledgements

The following sources aided with the development of this crate:\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/d/dc/Cosmic_rune_detail.png/800px-Cosmic_rune_detail.png?734d1" width="10"> &nbsp;[OpenRS](https://www.rune-server.ee/runescape-development/rs-503-client-server/downloads/312510-openrs-cache-library.html)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/f/f3/Air_rune_detail.png/800px-Air_rune_detail.png?b7f49" width="10"> &nbsp;[RuneLite](https://runelite.net/)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/0/0f/Law_rune_detail.png/800px-Law_rune_detail.png?dc1f3" width="10"> &nbsp;[OSRS Cache Parsing Blog](https://www.osrsbox.com/blog/2018/07/26/osrs-cache-research-extract-cache-definitions/)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/a/ae/Chaos_rune_detail.png/800px-Chaos_rune_detail.png?0d8cb" width="10"> &nbsp;[RSMod](https://github.com/Tomm0017/rsmod)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/8/8b/Soul_rune_detail.png/800px-Soul_rune_detail.png?75ada" width="10"> &nbsp;[Librsfs](https://github.com/Velocity-/librsfs)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/c/c1/Blood_rune_detail.png/800px-Blood_rune_detail.png?2cf9e" width="10"> &nbsp;[OSRSBox](https://www.osrsbox.com/)\
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://oldschool.runescape.wiki/images/thumb/7/72/Earth_rune_detail.png/800px-Earth_rune_detail.png?991bd" width="10"> &nbsp;[Jagex-Store-5](https://github.com/guthix/Jagex-Store-5)

## License

`osrs-cache` is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
