use rs2cache::{group::Group, js5_index::Js5IndexFile};
use std::collections::BTreeMap;

#[test]
fn test_unpack_single() {
    let actual = Group::unpack(
        vec![0, 1, 2, 3],
        &BTreeMap::from([(1, Js5IndexFile { name_hash: 0 })]),
    );
    let expected = BTreeMap::from([(1, vec![0, 1, 2, 3])]);

    assert_eq!(expected, actual);
}

#[test]
fn test_unpack_zero_stripes() {
    let expected = BTreeMap::from([(0, Vec::new()), (1, Vec::new()), (3, Vec::new())]);
    let actual = Group::unpack(
        vec![0],
        &BTreeMap::from([
            (0, Js5IndexFile { name_hash: 0 }),
            (1, Js5IndexFile { name_hash: 0 }),
            (3, Js5IndexFile { name_hash: 0 }),
        ]),
    );

    assert_eq!(expected, actual);
}

#[test]
fn test_unpack_one_stripe() {
    let expected = BTreeMap::from([
        (0, vec![0, 1, 2]),
        (1, vec![3, 4, 5, 6, 7]),
        (3, vec![8, 9]),
    ]);
    let actual = Group::unpack(
        vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0, 3, 0, 0, 0, 2, 0xFF, 0xFF, 0xFF, 0xFD, 1,
        ],
        &BTreeMap::from([
            (0, Js5IndexFile { name_hash: 0 }),
            (1, Js5IndexFile { name_hash: 0 }),
            (3, Js5IndexFile { name_hash: 0 }),
        ]),
    );

    assert_eq!(expected, actual);
}

#[test]
fn test_unpack_multiple_stripe() {
    let expected = BTreeMap::from([
        (0, vec![0, 1, 2]),
        (1, vec![3, 4, 5, 6, 7]),
        (3, vec![8, 9]),
    ]);
    let actual = Group::unpack(
        vec![
            0, 1, 3, 4, 8, 9, 2, 5, 6, 7, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            2, 0xFF, 0xFF, 0xFF, 0xFD, 2,
        ],
        &BTreeMap::from([
            (0, Js5IndexFile { name_hash: 0 }),
            (1, Js5IndexFile { name_hash: 0 }),
            (3, Js5IndexFile { name_hash: 0 }),
        ]),
    );

    assert_eq!(expected, actual);
}
