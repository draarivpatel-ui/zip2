//! A raw copy keeps the header fields the source entry recorded: the version
//! needed to extract and the general purpose bits describing the stored bytes.
//! Both used to be derived from the copy's own properties, so `raw_copy_file`
//! wrote a header that differed from the source's (see #1346).

use std::io::{Cursor, Write};

use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, ZipArchive};

/// Offset of "version needed to extract" in a local file header.
const LOCAL_VERSION_OFFSET: usize = 4;
/// Offset of the general purpose bit flag in a local file header.
const LOCAL_FLAGS_OFFSET: usize = 6;
/// Offset of "version needed to extract" in a central directory entry.
const CENTRAL_VERSION_OFFSET: usize = 6;
/// Offset of the general purpose bit flag in a central directory entry.
const CENTRAL_FLAGS_OFFSET: usize = 8;

/// A version higher than the one Stored needs, as some zippers record it.
const SOURCE_VERSION: u16 = 45;
/// An informational compression-level bit, set by some zippers.
const SOURCE_FLAGS: u16 = 0b0110;

fn source_archive() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "a.txt",
            SimpleFileOptions::DEFAULT.compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(b"hi").unwrap();
    let mut input = writer.finish().unwrap().into_inner();

    // Write the source's own "version needed to extract" and general purpose
    // bits into both headers, as a zipper that records them would.
    input[LOCAL_VERSION_OFFSET..LOCAL_VERSION_OFFSET + 2]
        .copy_from_slice(&SOURCE_VERSION.to_le_bytes());
    input[LOCAL_FLAGS_OFFSET..LOCAL_FLAGS_OFFSET + 2].copy_from_slice(&SOURCE_FLAGS.to_le_bytes());
    let central = input
        .windows(4)
        .position(|w| w == b"PK\x01\x02")
        .expect("central directory entry");
    input[central + CENTRAL_VERSION_OFFSET..central + CENTRAL_VERSION_OFFSET + 2]
        .copy_from_slice(&SOURCE_VERSION.to_le_bytes());
    input[central + CENTRAL_FLAGS_OFFSET..central + CENTRAL_FLAGS_OFFSET + 2]
        .copy_from_slice(&SOURCE_FLAGS.to_le_bytes());
    input
}

#[test]
fn raw_copy_keeps_version_needed_to_extract() {
    let input = source_archive();
    let mut archive = ZipArchive::new(Cursor::new(&input)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .raw_copy_file(archive.by_index_raw(0).unwrap())
        .unwrap();
    let output = writer.finish().unwrap().into_inner();

    assert_eq!(
        &output[LOCAL_VERSION_OFFSET..LOCAL_VERSION_OFFSET + 2],
        &SOURCE_VERSION.to_le_bytes(),
        "local header version needed to extract changed"
    );
    let central = output
        .windows(4)
        .position(|w| w == b"PK\x01\x02")
        .expect("central directory entry");
    assert_eq!(
        &output[central + CENTRAL_VERSION_OFFSET..central + CENTRAL_VERSION_OFFSET + 2],
        &SOURCE_VERSION.to_le_bytes(),
        "central version needed to extract changed"
    );
}

#[test]
fn raw_copy_keeps_stored_data_flag_bits() {
    let input = source_archive();
    let mut archive = ZipArchive::new(Cursor::new(&input)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .raw_copy_file(archive.by_index_raw(0).unwrap())
        .unwrap();
    let output = writer.finish().unwrap().into_inner();

    let local_flags = u16::from_le_bytes(
        output[LOCAL_FLAGS_OFFSET..LOCAL_FLAGS_OFFSET + 2]
            .try_into()
            .unwrap(),
    );
    assert_eq!(local_flags, SOURCE_FLAGS, "local header flags changed");
    let central = output
        .windows(4)
        .position(|w| w == b"PK\x01\x02")
        .expect("central directory entry");
    let central_flags = u16::from_le_bytes(
        output[central + CENTRAL_FLAGS_OFFSET..central + CENTRAL_FLAGS_OFFSET + 2]
            .try_into()
            .unwrap(),
    );
    assert_eq!(central_flags, SOURCE_FLAGS, "central flags changed");
}

#[test]
fn raw_copy_still_round_trips() {
    let input = source_archive();
    let mut archive = ZipArchive::new(Cursor::new(&input)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .raw_copy_file(archive.by_index_raw(0).unwrap())
        .unwrap();
    let output = writer.finish().unwrap().into_inner();

    let mut copied = ZipArchive::new(Cursor::new(output)).unwrap();
    let mut file = copied.by_name("a.txt").unwrap();
    let mut contents = String::new();
    std::io::Read::read_to_string(&mut file, &mut contents).unwrap();
    assert_eq!(contents, "hi");
}
