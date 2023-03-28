use std::path::Path;

use rs2cache::store::{disk_store::DiskStore, Store};

#[test]
fn test_list_groups() {
    read_test("single-block", |store| {
        assert_eq!(vec![1], store.list(255));
    });
    read_test("fragmented", |store| {
        assert_eq!(vec![0, 1], store.list(255));
    });
    read_test("single-block-extended", |store| {
        assert_eq!(vec![65536], store.list(255));
    });
}

// TODO: Handle this error
/*#[test]
fn test_list_non_existent() {
    read_test("empty", |store| {
        assert_eq!(vec![1], store.list(255));
    });
}*/

#[test]
fn test_read_single_block() {
    read_test("single-block", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".as_bytes();
        assert_eq!(expected, actual);
    });
}

#[test]
fn test_read_single_block_extended() {
    read_test("single-block-extended", |store| {
        let actual = store.read(255, 65536);
        let expected = "OpenRS2".as_bytes();
        assert_eq!(expected, actual);
    });
}

#[test]
fn test_read_two_blocks() {
    read_test("two-blocks", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(100).into_bytes();
        assert_eq!(expected, actual);
    });
}

#[test]
fn test_read_two_blocks_extended() {
    read_test("two-blocks-extended", |store| {
        let actual = store.read(255, 65536);
        let expected = "OpenRS2".repeat(100).into_bytes();
        assert_eq!(expected, actual);
    });
}

#[test]
fn test_read_multiple_blocks() {
    read_test("multiple-blocks", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(1000).into_bytes();
        assert_eq!(expected, actual);
    });
}

#[test]
fn test_read_multiple_blocks_extended() {
    read_test("multiple-blocks-extended", |store| {
        let actual = store.read(255, 65536);
        let expected = "OpenRS2".repeat(1000).into_bytes();
        assert_eq!(expected, actual);
    });
}

// TODO: Error handling here, simply follow the trace of error and handle accordingly
/*#[test]
fn test_read_non_existent() {
    read_test("single-block", |store| {
        store.read(0, 0);
        store.read(255, 0);
        store.read(255, 2);
    });
}*/

#[test]
fn test_read_fragmented() {
    read_test("fragmented", |store| {
        let actual = store.read(255, 1);
        let expected = "OpenRS2".repeat(100).into_bytes();
        assert_eq!(expected, actual);
    });
}

fn read_test<P, F>(p: P, f: F)
where
    P: AsRef<Path>,
    F: FnOnce(DiskStore),
{
    f(DiskStore::open(Path::new("tests/data/disk-store").join(p)))
}
