use rs2cache::{Cache, DiskStore, Store};
use std::path::Path;

#[test]
fn test_read_single_block() {
    read_test("single-block", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".as_bytes();
        assert_eq!(expected, actual);
    });
}

// TODO
/*#[test]
fn test_read_single_block_extended() {
    let store = read_test("single-block");
    let bytes = store.read(255, 1);
    assert_eq!("OpenRS2".as_bytes(), bytes);
}*/

#[test]
fn test_read_two_blocks() {
    read_test("two-blocks", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(100).into_bytes();
        assert_eq!(expected, actual);
    });
}

// TODO: Extended for two blocks too here

#[test]
fn test_read_multiple_blocks() {
    read_test("multiple-blocks", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(1000).into_bytes();
        assert_eq!(expected, actual);
    });
}

// TODO: Extended for multiple blocks too here

// Error handling here, simply follow the trace of error aand handle accordingly
/*#[test]
fn test_read_non_existent() {
    let store = read_test("single-block");
    let actual = store.read(0, 0);
    assert_eq!(0, 0);
}*/

#[test]
fn test_read_fragmented() {
    read_test("fragmented", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(100).into_bytes();
        assert_eq!(expected, actual);
    });
}

fn read_test<T, F>(str: T, func: F)
where
    T: AsRef<str>,
    F: FnOnce(DiskStore),
{
    func(DiskStore::open(
        Path::new("tests/data/disk-store")
            .join(str.as_ref())
            .to_str()
            .unwrap(),
    ))
}
