//! Validator for the cache.
//!
//! # Example
//!
//! ```
//! # use osrscache::Cache;
//! use osrscache::checksum::{Checksum};
//!
//! # fn main() -> osrscache::Result<()> {
//! # let cache = Cache::new("./data/cache")?;
//! let checksum = Checksum::new(&cache)?;
//!
//! // Encode the checksum with the OSRS protocol.
//! let buffer = checksum.encode()?;
//! # Ok(())
//! # }
//! ```

use std::iter::IntoIterator;
use std::slice::Iter;

use crate::{codec, codec::Compression, error::ValidateError, Cache, REFERENCE_TABLE};
use crc::{Crc, CRC_32_ISO_HDLC};
use nom::{combinator::cond, number::complete::be_u32};

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// Contains index validation data.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(feature = "serde-derive", derive(Serialize, Deserialize))]
pub struct Entry {
    pub crc: u32,
    pub version: u32,
}

// TODO: fix documentation
/// Validator for the `Cache`.
///
/// Used to validate cache index files. It contains a list of entries, one entry for each index file.
///
/// In order to create the `Checksum` the
/// [create_checksum()](../struct.Cache.html#method.create_checksum) function has to be
/// called on `Cache`.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(feature = "serde-derive", derive(Serialize, Deserialize))]
pub struct Checksum<'a> {
    index_count: usize,
    entries: Vec<Entry>,
    rsa_keys: Option<RsaKeys<'a>>,
}

impl<'a> Checksum<'a> {
    pub fn new(cache: &Cache) -> crate::Result<Self> {
        Self::new_internal(cache, None)
    }

    fn new_internal(cache: &Cache, rsa_keys: Option<RsaKeys<'a>>) -> crate::Result<Self> {
        let entries: Vec<Entry> = (0..cache.indices.len())
            .into_iter()
            .filter_map(|idx_id| cache.read(REFERENCE_TABLE, idx_id as u32).ok())
            .enumerate()
            .map(|(idx_id, buffer)| -> crate::Result<Entry> {
                if buffer.is_empty() || idx_id == 47 {
                    Ok(Entry::default())
                } else {
                    let data = codec::decode(&buffer)?;
                    let (_, version) = cond(data[0] >= 6, be_u32)(&data[1..5])?;
                    let version = version.unwrap_or(0);

                    let mut digest = CRC.digest();
                    digest.update(&buffer);

                    Ok(Entry {
                        crc: digest.finalize(),
                        version,
                    })
                }
            })
            .filter_map(crate::Result::ok)
            .collect();

        Ok(Self {
            index_count: cache.indices.len(),
            entries,
            rsa_keys,
        })
    }

    /// Consumes the `Checksum` and encodes it into a byte buffer.
    ///
    ///
    /// Note: It defaults to OSRS.
    /// network traffic, which includes the checksum.
    /// first call [`with_rsa_keys`](struct.Checksum.html#method.with_rsa_keys) to make
    /// the checksum aware of the clients keys.
    ///
    /// After encoding the checksum it can be sent to the client.
    ///
    /// # Errors
    ///
    /// Returns a `CacheError` if the encoding fails.

    pub fn encode(self) -> crate::Result<Vec<u8>> {
        match self.rsa_keys {
            Some(_) => {
                unreachable!()
            }
            None => self.encode_osrs(),
        }
    }

    // TODO: documentation and write fail tests for this.
    /// Validates crcs with internal crcs.
    ///
    /// Only returns `true` if both the length of the iterators are the same
    /// and all of its elements are `eq`.

    pub fn validate<'b, I>(&self, crcs: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = &'b u32>,
    {
        let mut crcs = crcs.into_iter();
        let crcs_len = crcs.by_ref().count();
        if self.entries.len() != crcs_len {
            return Err(ValidateError::InvalidLength(self.entries.len(), crcs_len).into());
        }

        for (index, (internal, external)) in self
            .entries
            .iter()
            .map(|entry| &entry.crc)
            .zip(crcs)
            .enumerate()
        {
            if internal != external {
                return Err(ValidateError::InvalidCrc(*internal, *external, index).into());
            }
        }

        Ok(())
    }

    fn encode_osrs(self) -> crate::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(self.entries.len() * 8);

        for entry in self.entries {
            buffer.extend(&u32::to_be_bytes(entry.crc));
            buffer.extend(&u32::to_be_bytes(entry.version));
        }

        codec::encode(Compression::None, &buffer, None)
    }

    pub const fn index_count(&self) -> usize {
        self.index_count
    }

    pub fn iter(&self) -> Iter<'_, Entry> {
        self.entries.iter()
    }
}

// TODO: documentation
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(feature = "serde-derive", derive(Serialize, Deserialize))]
pub struct RsaKeys<'a> {
    pub exponent: &'a [u8],
    pub modulus: &'a [u8],
}

impl<'a> RsaKeys<'a> {
    pub const fn new(exponent: &'a [u8], modulus: &'a [u8]) -> Self {
        Self { exponent, modulus }
    }
}

// impl IntoIterator for Checksum {
//     type Item = Entry;
//     type IntoIter = std::vec::IntoIter<Entry>;

//
//     fn into_iter(self) -> Self::IntoIter {
//         self.entries.into_iter()
//     }
// }

// impl<'a> IntoIterator for &'a Checksum {
//     type Item = &'a Entry;
//     type IntoIter = Iter<'a, Entry>;

//
//     fn into_iter(self) -> Self::IntoIter {
//         self.entries.iter()
//     }
// }
