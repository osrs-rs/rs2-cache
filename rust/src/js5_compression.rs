use crate::xtea::xtea_decipher;
use bzip2::read::BzDecoder;
use flate2::bufread::GzDecoder;
use lzma_rs::{decompress, lzma_decompress_with_options};
use osrs_bytes::ReadExt;
use std::io::{Cursor, Read};

const COMPRESSION_TYPE_NONE: u8 = 0;
const COMPRESSION_TYPE_BZIP: u8 = 1;
const COMPRESSION_TYPE_GZIP: u8 = 2;
const COMPRESSION_TYPE_LZMA: u8 = 3;
pub struct Js5Compression {}

impl Js5Compression {
    pub fn uncompress<T: AsRef<[u8]>>(input: T, xtea_keys: Option<[u32; 4]>) -> Vec<u8> {
        let mut input_ref = input.as_ref();

        if input_ref.as_ref().len() < 5 {
            panic!("Missing header");
        }

        let type_id = input_ref.read_u8().unwrap();
        // TODO: Check if type_id is correct here and panic if not or just like throw an error and return here

        let len = input_ref.read_i32().unwrap();
        if len < 0 {
            panic!("Length is negative {len}");
        }

        if type_id == COMPRESSION_TYPE_NONE {
            if input_ref.len() < len as usize {
                panic!("Data truncated");
            }

            if let Some(xtea_keys) = xtea_keys {
                return xtea_decipher(input_ref, &xtea_keys);
            }

            return input_ref[..len as usize].to_vec();
        }

        let len_with_uncompressed_len = len + 4;
        if input_ref.len() < len_with_uncompressed_len as usize {
            panic!("Data truncated");
        }

        let plain_text = Self::decrypt(input_ref, len_with_uncompressed_len, xtea_keys);
        let mut plain_text_csr = Cursor::new(plain_text);

        let uncompressed_len = plain_text_csr.read_i32().unwrap();
        if uncompressed_len < 0 {
            panic!("Uncompressed length is negative: {uncompressed_len}");
        }

        // Copy bytes from the cursor to a buffer skipping over already read ones
        let mut plain_text =
            vec![0; plain_text_csr.get_ref().len() - plain_text_csr.position() as usize];

        plain_text_csr.read_exact(&mut plain_text).unwrap();

        // Skip version by using len
        let input_stream = &plain_text[..len as usize];

        match type_id {
            COMPRESSION_TYPE_BZIP => {
                decompress_archive_bzip2(input_stream, uncompressed_len as u32)
            }
            COMPRESSION_TYPE_GZIP => decompress_archive_gzip(input_stream, uncompressed_len as u32),
            COMPRESSION_TYPE_LZMA => decompress_archive_lzma(input_stream, uncompressed_len as u32),
            _ => panic!("Unknown compression type {type_id}"),
        }
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
fn decompress_archive_bzip2<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut compressed_data = Vec::with_capacity(archive_data.as_ref().len() + 4);
    compressed_data.extend(b"BZh1");
    compressed_data.extend(archive_data.as_ref());

    let mut decompressor = BzDecoder::new(compressed_data.as_slice());

    decompressor.read_exact(&mut decompressed_data).unwrap();
    decompressed_data
}

// Decompress using gzip
fn decompress_archive_gzip<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decompressed_data = vec![0; decompressed_size as usize];

    let mut decompressor = GzDecoder::new(archive_data.as_ref());
    decompressor.read_exact(&mut decompressed_data).unwrap();

    decompressed_data
}

// Decompress using lzma
fn decompress_archive_lzma<T: AsRef<[u8]>>(archive_data: T, decompressed_size: u32) -> Vec<u8> {
    let mut decomp: Vec<u8> = Vec::new();

    lzma_decompress_with_options(
        &mut archive_data.as_ref(),
        &mut decomp,
        &decompress::Options {
            unpacked_size: decompress::UnpackedSize::UseProvided(Some(decompressed_size as u64)),
            memlimit: None,
            allow_incomplete: false,
        },
    )
    .unwrap();

    decomp
}
