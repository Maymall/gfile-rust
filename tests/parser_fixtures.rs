// SPDX-License-Identifier: GPL-3.0-only

use gfile_rust::{error::GfileError, parser::download::parse_single_file_page};

const FILE_ID: &str = "0123abcd-000000example";

#[test]
fn parse_single_basic_fixture_extracts_file_info() {
    let page = parse_single_file_page(include_str!("fixtures/single_basic.html"), FILE_ID).unwrap();

    assert_eq!(page.files.len(), 1);
    let file = &page.files[0];
    assert_eq!(file.file_id, FILE_ID);
    assert_eq!(file.raw_name, "example file.bin");
    assert_eq!(file.display_size.as_deref(), Some("10KB"));
    assert_eq!(file.approx_bytes, Some(10 * 1024));
}

#[test]
fn parse_single_japanese_fixture_preserves_name_bytes() {
    let page =
        parse_single_file_page(include_str!("fixtures/single_japanese.html"), FILE_ID).unwrap();

    assert_eq!(
        page.files[0].raw_name.as_bytes(),
        "テスト資料_2026.zip".as_bytes()
    );
}

#[test]
fn parse_single_broken_fixture_reports_missing_dl() {
    let error = parse_single_file_page(include_str!("fixtures/single_broken.html"), FILE_ID)
        .expect_err("broken fixture should fail");

    match error {
        GfileError::Parse { what, .. } => assert!(what.contains("#dl"), "{what}"),
        other => panic!("unexpected error: {other:?}"),
    }
}
