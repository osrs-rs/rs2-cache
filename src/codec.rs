//! (De)compression and enciphering/deciphering.
//!
//! ```
//! # use osrscache::Cache;
//! use osrscache::codec::{ self, Compression };
//!
//! # fn main() -> osrscache::Result<()> {
//! # let cache = Cache::new("./data/cache")?;
//! let buffer = cache.read(2, 10)?;
//!
//! let decompressed_buffer = codec::decode(&buffer)?;
//! //let compressed_buffer = codec::encode(Compression::Bzip2, &decompressed_buffer, None)?; // TODO: No encoding supported in bzip2-rs yet, await this
//! # Ok(())
//! # }
//! ```

use bzip2_rs::DecoderReader;
use flate2::{bufread::GzDecoder, write::GzEncoder};
use nom::{
    combinator::cond,
    number::complete::{be_i16, be_u32, be_u8},
};
use std::convert::TryFrom;
use std::io::{self, Read, Write};

use crate::{
    error::{CacheError, CompressionError},
    util::xtea,
};

/// Supported compression types for RuneScape.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Compression {
    None,
    Bzip2,
    Gzip,
}

/// Decodes a cache buffer with some additional data.
///
/// This struct can be useful if you need more details then just the decoded data.
///
/// # Examples
///
/// `TryFrom<&[u8]> -> DecodedBuffer`:  
/// ```
/// # use osrscache::Cache;
/// # use osrscache::codec::Compression;
/// use osrscache::codec::DecodedBuffer;
/// use std::convert::TryFrom;
///
/// # fn main() -> osrscache::Result<()> {
/// # let cache = Cache::new("./data/cache")?;
/// let buffer = cache.read(2, 10)?;
/// let decoded = DecodedBuffer::try_from(buffer.as_slice())?;
///
/// assert_eq!(decoded.compression, Compression::Bzip2);
/// assert_eq!(decoded.len, 886570);
/// assert_eq!(decoded.version, Some(12609));
/// # Ok(())
/// # }
/// ```
///
/// Getting the inner buffer:
/// This conversion is free.
/// ```
/// # use osrscache::Cache;
/// # use osrscache::codec::Compression;
/// # use osrscache::codec::DecodedBuffer;
/// # use std::convert::TryFrom;
/// # fn main() -> osrscache::Result<()> {
/// # let cache = Cache::new("./data/cache")?;
/// let buffer = cache.read(2, 10)?;
/// let decoded = DecodedBuffer::try_from(buffer.as_slice())?;
///
/// let inner_buffer: Vec<u8> = decoded.into_vec();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct DecodedBuffer {
    pub compression: Compression,
    pub len: usize,
    pub version: Option<i16>,
    buffer: Vec<u8>,
}

impl DecodedBuffer {
    /// Free conversion from `DecodedBuffer` into `Vec<u8>`.
    // False positive, issue already open but not being worked on atm.
    #[allow(clippy::missing_const_for_fn)]

    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}

/// Encodes a buffer, with the selected `Compression` format. version is an optional argument
/// that encodes the version of this buffer into it, if no version should be encoded
/// pass None.
///
/// The following process takes place when encoding:
/// 1. Compress the buffer with the selected compression format.
/// 2. Allocate a new buffer.
/// 3. Push the compression type as a byte into the new buffer.
/// 4. Push the length (u32) into the buffer of the compressed data from step 1.
/// 5. If a compression type was selected (and not `Compression::None`) insert the uncompressed length as u32.
/// 6. Extend the buffer with the compressed data.
/// 7. Add the `version` as i16 if present.
/// 8. Encode complete.
///
/// **NOTE: When compressing with gzip the header is removed
/// before the compressed data is returned.
/// The encoded buffer will not contain the gzip header.**
///
/// # Errors
///
/// Returns an error if the data couldn't be compressed or is invalid.

pub fn encode(
    compression: Compression,
    data: &[u8],
    version: Option<i16>,
) -> crate::Result<Vec<u8>> {
    encode_internal(compression, data, version, None)
}

/// Encodes the buffer with the given XTEA keys.
///
/// For more details see [`encode`](encode)

pub fn encode_with_keys(
    compression: Compression,
    data: &[u8],
    version: Option<i16>,
    keys: &[u32; 4],
) -> crate::Result<Vec<u8>> {
    encode_internal(compression, data, version, Some(keys))
}

fn encode_internal(
    compression: Compression,
    data: &[u8],
    version: Option<i16>,
    keys: Option<&[u32; 4]>,
) -> crate::Result<Vec<u8>> {
    let mut compressed_data = match compression {
        Compression::None => data.to_owned(),
        Compression::Bzip2 => todo!(), //compress_bzip2(data)?,
        Compression::Gzip => compress_gzip(data)?,
    };

    if let Some(keys) = keys {
        compressed_data = xtea::encipher(&compressed_data, keys);
    }

    let mut buffer = Vec::with_capacity(compressed_data.len() + 11);
    buffer.push(compression as u8);
    buffer.extend(&u32::to_be_bytes(compressed_data.len() as u32));
    if compression != Compression::None {
        buffer.extend(&u32::to_be_bytes(data.len() as u32));
    }

    buffer.extend(compressed_data);

    if let Some(version) = version {
        buffer.extend(&i16::to_be_bytes(version));
    }

    Ok(buffer)
}

/// Decodes the buffer.
///
/// The following process takes place when decoding:
/// 1. Read the first byte to determine which compression type should be used to decompress.
/// 2. Read the length of the rest of the buffer.
/// 3. Decompress the remaining bytes.
///
/// # Errors
///
/// Returns an error if the remaining bytes couldn't be decompressed.

pub fn decode(buffer: &[u8]) -> crate::Result<Vec<u8>> {
    Ok(DecodedBuffer::try_from(buffer)?.into_vec())
}

/// Decodes the buffer with the given XTEA keys.
///
/// For more details see [`decode`](decode)

pub fn decode_with_keys(buffer: &[u8], keys: &[u32; 4]) -> crate::Result<Vec<u8>> {
    Ok(decode_internal(buffer, Some(keys))?.into_vec())
}

fn decode_internal(buffer: &[u8], keys: Option<&[u32; 4]>) -> crate::Result<DecodedBuffer> {
    let (buffer, compression) = be_u8(buffer)?;
    let compression = Compression::try_from(compression)?;

    let (buffer, compressed_len) = be_u32(buffer)?;
    let compressed_len = compressed_len as usize;

    let buffer = keys.map_or_else(|| buffer.to_vec(), |keys| xtea::decipher(buffer, keys));

    let (decompressed_len, version, buffer) = match compression {
        Compression::None => decompress_none(&buffer, compressed_len)?,
        Compression::Bzip2 => decompress_bzip2(&buffer, compressed_len)?,
        Compression::Gzip => decompress_gzip(&buffer, compressed_len)?,
    };

    Ok(DecodedBuffer {
        compression,
        len: decompressed_len,
        version,
        buffer,
    })
}

/*
TODO: Disabled while bzip2-rs does not have encoding support.
This should be enabled once bzip2-rs has support for it (it is an upcoming feature as per their README)
fn compress_bzip2(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut compressor = BzEncoder::new(Vec::new(), bzip2::Compression::fast());
    compressor.write_all(data)?;
    let mut compressed_data = compressor.finish()?;
    compressed_data.drain(..4);

    Ok(compressed_data)
}*/

fn compress_gzip(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut compressor = GzEncoder::new(Vec::new(), flate2::Compression::best());
    compressor.write_all(data)?;
    let compressed_data: Vec<u8> = compressor.finish()?;

    Ok(compressed_data)
}

fn decompress_none(buffer: &[u8], len: usize) -> crate::Result<(usize, Option<i16>, Vec<u8>)> {
    let mut compressed_data = vec![0; len];
    compressed_data.copy_from_slice(buffer);

    let (_, version) = cond(buffer.len() - len >= 2, be_i16)(buffer)?;

    Ok((len, version, compressed_data))
}

fn decompress_bzip2(buffer: &[u8], len: usize) -> crate::Result<(usize, Option<i16>, Vec<u8>)> {
    let (buffer, decompressed_len) = be_u32(buffer)?;
    let mut compressed_data = vec![0; len];
    compressed_data[4..len].copy_from_slice(&buffer[..len - 4]);
    compressed_data[..4].copy_from_slice(b"BZh1");

    let (_, version) = cond(buffer.len() - len >= 2, be_i16)(buffer)?;

    let mut decompressor = DecoderReader::new(compressed_data.as_slice());
    let mut decompressed_data = vec![0; decompressed_len as usize];
    decompressor.read_exact(&mut decompressed_data)?;

    Ok((decompressed_len as usize, version, decompressed_data))
}

fn decompress_gzip(buffer: &[u8], len: usize) -> crate::Result<(usize, Option<i16>, Vec<u8>)> {
    let (buffer, decompressed_len) = be_u32(buffer)?;
    let mut compressed_data = vec![0; len];
    compressed_data.copy_from_slice(&buffer[..len]);

    let (_, version) = cond(buffer.len() - len >= 2, be_i16)(buffer)?;

    let mut decompressor = GzDecoder::new(compressed_data.as_slice());
    let mut decompressed_data = vec![0; decompressed_len as usize];
    decompressor.read_exact(&mut decompressed_data)?;

    Ok((decompressed_len as usize, version, decompressed_data))
}

impl Default for Compression {
    fn default() -> Self {
        Self::None
    }
}

impl From<Compression> for u8 {
    fn from(compression: Compression) -> Self {
        match compression {
            Compression::None => 0,
            Compression::Bzip2 => 1,
            Compression::Gzip => 2,
        }
    }
}

impl TryFrom<u8> for Compression {
    type Error = CompressionError;

    fn try_from(compression: u8) -> Result<Self, Self::Error> {
        match compression {
            0 => Ok(Self::None),
            1 => Ok(Self::Bzip2),
            2 => Ok(Self::Gzip),
            _ => Err(CompressionError::Unsupported(compression)),
        }
    }
}

impl TryFrom<&[u8]> for DecodedBuffer {
    type Error = CacheError;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        decode_internal(buffer, None)
    }
}
