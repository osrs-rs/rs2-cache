use crate::xtea::xtea_decipher;
use bzip2::read::BzDecoder;
use flate2::bufread::GzDecoder;
use lzma_rs::{decompress, lzma_decompress_with_options};
use osrs_bytes::ReadExt;
use std::io::{Cursor, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Js5CompressionError {
    #[error("missing header")]
    MissingHeader,
    #[error("negative length: {0}")]
    NegativeLength(i32),
    #[error("data truncated")]
    DataTruncated,
    #[error("uncompressed length is negative: {0}")]
    UncompressedLengthIsNegative(i32),
    #[error("unknown compression type: {0}")]
    UnknownCompressionType(u8),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lzma error: {0}")]
    Lzma(#[from] lzma_rs::error::Error),
}

const COMPRESSION_TYPE_NONE: u8 = 0;
const COMPRESSION_TYPE_BZIP: u8 = 1;
const COMPRESSION_TYPE_GZIP: u8 = 2;
const COMPRESSION_TYPE_LZMA: u8 = 3;
pub struct Js5Compression {}

impl Js5Compression {
    pub fn uncompress<T: AsRef<[u8]>>(
        input: T,
        xtea_keys: Option<[u32; 4]>,
    ) -> Result<Vec<u8>, Js5CompressionError> {
        let mut input_ref = input.as_ref();

        if input_ref.as_ref().len() < 5 {
            return Err(Js5CompressionError::MissingHeader);
        }

        let type_id = input_ref.read_u8()?;
        // TODO: Check if type_id is correct here and error if not in range 0-3

        let len = input_ref.read_i32()?;
        if len < 0 {
            return Err(Js5CompressionError::NegativeLength(len));
        }

        if type_id == COMPRESSION_TYPE_NONE {
            if input_ref.len() < len as usize {
                return Err(Js5CompressionError::DataTruncated);
            }

            if let Some(xtea_keys) = xtea_keys {
                return Ok(xtea_decipher(input_ref, &xtea_keys));
            }

            return Ok(input_ref[..len as usize].to_vec());
        }

        let len_with_uncompressed_len = len + 4;
        if input_ref.len() < len_with_uncompressed_len as usize {
            return Err(Js5CompressionError::DataTruncated);
        }

        let plain_text = Self::decrypt(input_ref, len_with_uncompressed_len, xtea_keys);
        let mut plain_text_csr = Cursor::new(plain_text);

        let uncompressed_len = plain_text_csr.read_i32()?;
        if uncompressed_len < 0 {
            return Err(Js5CompressionError::UncompressedLengthIsNegative(
                uncompressed_len,
            ));
        }

        // Copy bytes from the cursor to a buffer skipping over already read ones
        let mut plain_text =
            vec![0; plain_text_csr.get_ref().len() - plain_text_csr.position() as usize];

        plain_text_csr.read_exact(&mut plain_text)?;

        // Skip version by using len
        let input_stream = &plain_text[..len as usize];

        let decomp = match type_id {
            COMPRESSION_TYPE_BZIP => {
                decompress_archive_bzip2(input_stream, uncompressed_len as u32)
            }
            COMPRESSION_TYPE_GZIP => decompress_archive_gzip(input_stream, uncompressed_len as u32),
            COMPRESSION_TYPE_LZMA => decompress_archive_lzma(input_stream, uncompressed_len as u32),
            _ => return Err(Js5CompressionError::UnknownCompressionType(type_id)),
        }?;

        Ok(decomp)
    }

    fn decrypt<T: AsRef<[u8]>>(input: T, len: i32, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        if let Some(xtea_keys) = xtea_keys {
            xtea_decipher(input.as_ref(), &xtea_keys)
        } else {
            input.as_ref().to_vec()[..len as usize].to_vec()
        }
    }
}

// Decompress using bzip2
fn decompress_archive_bzip2<T: AsRef<[u8]>>(
    archive_data: T,
    decompressed_size: u32,
) -> Result<Vec<u8>, Js5CompressionError> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut compressed_data = Vec::with_capacity(archive_data.as_ref().len() + 4);
    compressed_data.extend(b"BZh1");
    compressed_data.extend(archive_data.as_ref());

    let mut decompressor = BzDecoder::new(compressed_data.as_slice());

    decompressor.read_exact(&mut decompressed_data)?;
    Ok(decompressed_data)
}

// Decompress using gzip
fn decompress_archive_gzip<T: AsRef<[u8]>>(
    archive_data: T,
    decompressed_size: u32,
) -> Result<Vec<u8>, Js5CompressionError> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut decompressor = GzDecoder::new(archive_data.as_ref());
    decompressor.read_exact(&mut decompressed_data)?;

    Ok(decompressed_data)
}

// Decompress using lzma
fn decompress_archive_lzma<T: AsRef<[u8]>>(
    archive_data: T,
    decompressed_size: u32,
) -> Result<Vec<u8>, Js5CompressionError> {
    let mut decomp: Vec<u8> = Vec::new();

    lzma_decompress_with_options(
        &mut archive_data.as_ref(),
        &mut decomp,
        &decompress::Options {
            unpacked_size: decompress::UnpackedSize::UseProvided(Some(decompressed_size as u64)),
            memlimit: None,
            allow_incomplete: false,
        },
    )?;

    Ok(decomp)
}

#[cfg(test)]
mod tests {
    use super::*;

    use memmap2::Mmap;
    use std::{fs::File, path::Path};

    #[test]
    fn test_uncompress_none() {
        read("none.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, None).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_gzip() {
        read("gzip.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, None).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_large_gzip() {
        read("gzip-large.dat", |input| {
            read("large.dat", |expected| {
                assert_eq!(
                    expected.to_vec(),
                    Js5Compression::uncompress(input, None).unwrap()
                )
            });
        });
    }

    #[test]
    fn test_uncompress_bzip2() {
        read("bzip2.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, None).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_lzma() {
        read("lzma.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, None).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_none_encrypted() {
        read("none-encrypted.dat", |data| {
            assert_eq!(
                "OpenRS2".repeat(3).as_bytes(),
                Js5Compression::uncompress(data, Some(KEY)).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_gzip_encrypted() {
        read("gzip-encrypted.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, Some(KEY)).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_bzip2_encrypted() {
        read("bzip2-encrypted.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, Some(KEY)).unwrap()
            );
        });
    }

    #[test]
    fn test_uncompress_lzma_encrypted() {
        read("lzma-encrypted.dat", |data| {
            assert_eq!(
                "OpenRS2".as_bytes(),
                Js5Compression::uncompress(data, Some(KEY)).unwrap()
            );
        });
    }

    #[test]
    fn test_invalid_type() {
        read("invalid-type.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::UnknownCompressionType(4))
            ));
        });
    }

    #[test]
    fn test_invalid_length() {
        read("invalid-length.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::NegativeLength(-2147483648))
            ));
        });
    }

    #[test]
    fn test_invalid_uncompressed_length() {
        read("invalid-uncompressed-length.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::UncompressedLengthIsNegative(
                    -2147483648
                ))
            ));
        });
    }

    #[test]
    fn test_none_eof() {
        read("none-eof.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::DataTruncated)
            ));
        });
    }

    #[test]
    fn test_bzip2_eof() {
        read("bzip2-eof.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Io(_))
            ));
        });
    }

    #[test]
    fn test_gzip_eof() {
        read("gzip-eof.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Io(_))
            ));
        });
    }

    #[test]
    fn test_lzma_eof() {
        read("lzma-eof.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Lzma(lzma_rs::error::Error::IoError(_)))
            ));
        });
    }

    #[test]
    fn test_bzip2_corrupt() {
        read("bzip2-corrupt.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Io(_))
            ));
        });
    }

    #[test]
    fn test_gzip_corrupt() {
        read("gzip-corrupt.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Io(_))
            ));
        });
    }

    #[test]
    fn test_lzma_corrupt() {
        read("lzma-corrupt.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::Lzma(lzma_rs::error::Error::LzmaError(
                    _
                )))
            ));
        });
    }

    #[test]
    fn test_missing_header() {
        read("missing-header.dat", |data| {
            assert!(matches!(
                Js5Compression::uncompress(data, None),
                Err(Js5CompressionError::MissingHeader)
            ));
        });
    }

    fn read<P, F>(p: P, f: F)
    where
        P: AsRef<Path>,
        F: FnOnce(Mmap),
    {
        f(
            unsafe { Mmap::map(&File::open(Path::new("tests/data/compression").join(p)).unwrap()) }
                .unwrap(),
        )
    }

    const KEY: [u32; 4] = [0x00112233, 0x44556677, 0x8899AABB, 0xCCDDEEFF];
    //const INVALID_KEY: [u32; 4] = [0x01234567, 0x89ABCDEF, 0x01234567, 0x89ABCDEF];
}
