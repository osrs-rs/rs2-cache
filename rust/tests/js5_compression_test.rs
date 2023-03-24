use memmap2::Mmap;
use rs2cache::Js5Compression;
use std::fs::File;
use std::path::Path;

#[test]
fn test_uncompress_none() {
    read_test("none.dat", |data| {
        assert_eq!(Js5Compression::uncompress(data, None), "OpenRS2".as_bytes());
    });
}

#[test]
fn test_uncompress_gzip() {
    read_test("gzip.dat", |data| {
        assert_eq!(Js5Compression::uncompress(data, None), "OpenRS2".as_bytes());
    });
}

#[test]
fn test_uncompress_large_gzip() {
    read_test("gzip-large.dat", |input| {
        read_test("large.dat", |expected| {
            assert_eq!(expected.to_vec(), Js5Compression::uncompress(input, None))
        });
    });
}

#[test]
fn test_uncompress_bzip2() {
    read_test("bzip2.dat", |data| {
        assert_eq!(Js5Compression::uncompress(data, None), "OpenRS2".as_bytes());
    });
}

#[test]
fn test_uncompress_none_encrypted() {
    read_test("none-encrypted.dat", |data| {
        assert_eq!(
            Js5Compression::uncompress(data, Some(KEY)),
            "OpenRS2".repeat(3).as_bytes()
        );
    });
}

#[test]
fn test_uncompress_gzip_encrypted() {
    read_test("gzip-encrypted.dat", |data| {
        assert_eq!(
            Js5Compression::uncompress(data, Some(KEY)),
            "OpenRS2".as_bytes()
        );
    });
}

#[test]
fn test_uncompress_bzip2_encrypted() {
    read_test("bzip2-encrypted.dat", |data| {
        assert_eq!(
            Js5Compression::uncompress(data, Some(KEY)),
            "OpenRS2".as_bytes()
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

fn read_test<T, F>(p: T, f: F)
where
    T: AsRef<Path>,
    F: FnOnce(Mmap),
{
    f(
        unsafe { Mmap::map(&File::open(Path::new("tests/data/compression").join(p)).unwrap()) }
            .unwrap(),
    )
}

const KEY: [u32; 4] = [0x00112233, 0x44556677, 0x8899AABB, 0xCCDDEEFF];
