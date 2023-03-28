use memmap2::Mmap;
use rs2cache::js5_compression::Js5Compression;
use std::{fs::File, path::Path};

#[test]
fn test_uncompress_none() {
    read("none.dat", |data| {
        assert_eq!("OpenRS2".as_bytes(), Js5Compression::uncompress(data, None));
    });
}

#[test]
fn test_uncompress_gzip() {
    read("gzip.dat", |data| {
        assert_eq!("OpenRS2".as_bytes(), Js5Compression::uncompress(data, None));
    });
}

#[test]
fn test_uncompress_large_gzip() {
    read("gzip-large.dat", |input| {
        read("large.dat", |expected| {
            assert_eq!(expected.to_vec(), Js5Compression::uncompress(input, None))
        });
    });
}

#[test]
fn test_uncompress_bzip2() {
    read("bzip2.dat", |data| {
        assert_eq!("OpenRS2".as_bytes(), Js5Compression::uncompress(data, None));
    });
}

#[test]
fn test_uncompress_lzma() {
    read("lzma.dat", |data| {
        assert_eq!("OpenRS2".as_bytes(), Js5Compression::uncompress(data, None));
    });
}

#[test]
fn test_uncompress_none_encrypted() {
    read("none-encrypted.dat", |data| {
        assert_eq!(
            "OpenRS2".repeat(3).as_bytes(),
            Js5Compression::uncompress(data, Some(KEY))
        );
    });
}

#[test]
fn test_uncompress_gzip_encrypted() {
    read("gzip-encrypted.dat", |data| {
        assert_eq!(
            "OpenRS2".as_bytes(),
            Js5Compression::uncompress(data, Some(KEY))
        );
    });
}

#[test]
fn test_uncompress_bzip2_encrypted() {
    read("bzip2-encrypted.dat", |data| {
        assert_eq!(
            "OpenRS2".as_bytes(),
            Js5Compression::uncompress(data, Some(KEY))
        );
    });
}

#[test]
fn test_uncompress_lzma_encrypted() {
    read("lzma-encrypted.dat", |data| {
        assert_eq!(
            "OpenRS2".as_bytes(),
            Js5Compression::uncompress(data, Some(KEY))
        );
    });
}

// Impl this later once Results are implemented
/*#[test]
fn test_bzip2_eof() {
    read_test("bzip2-eof.dat", |data| {
        assert_eq!(Js5Compression::uncompress(data, None), "OpenRS2".as_bytes());
    });
}*/

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
