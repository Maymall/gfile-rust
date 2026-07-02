// SPDX-License-Identifier: GPL-3.0-only

use scraper::{Html, Selector};
use tracing::debug;

use crate::{error::GfileError, parser::size::parse_display_size};

// gfile.py@4c45392 lines 255-258: single-file pages use URL path as file id,
// `.dl_size` text for display size, and `#dl` text for the web filename.
const FILE_NAME_SELECTOR: &str = "#dl";
const SIZE_SELECTOR: &str = ".dl_size";
const MATOMETE_SELECTOR: &str = "#contents_matomete";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub kind: PageKind,
    pub files: Vec<RemoteFile>,
    pub needs_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Single,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub file_id: String,
    pub raw_name: String,
    pub display_size: Option<String>,
    pub approx_bytes: Option<u64>,
}

pub fn parse_single_file_page(html: &str, file_id: &str) -> Result<PageInfo, GfileError> {
    let document = Html::parse_document(html);

    if select_first_text(&document, MATOMETE_SELECTOR)?.is_some() {
        return Err(parse_error(
            "matomete pages are not implemented in M1",
            "This build only supports single-file pages; matomete support is scheduled for M2.",
        ));
    }

    let raw_name = select_first_text(&document, FILE_NAME_SELECTOR)?
        .ok_or_else(|| parse_error("missing #dl", parse_hint()))?;
    debug!(raw_name = ?raw_name, "parsed raw_name");

    let display_size = select_first_text(&document, SIZE_SELECTOR)?
        .ok_or_else(|| parse_error("missing .dl_size", parse_hint()))?;
    let approx_bytes = parse_display_size(&display_size);

    Ok(PageInfo {
        kind: PageKind::Single,
        files: vec![RemoteFile {
            file_id: file_id.to_owned(),
            raw_name,
            display_size: Some(display_size),
            approx_bytes,
        }],
        needs_key: false,
    })
}

fn select_first_text(document: &Html, selector: &str) -> Result<Option<String>, GfileError> {
    let selector = Selector::parse(selector).map_err(|_| {
        parse_error(
            format!("invalid selector {selector}"),
            "This is an internal parser bug; please report it.",
        )
    })?;

    Ok(document
        .select(&selector)
        .next()
        .map(|node| node.text().collect::<String>().trim().to_owned()))
}

fn parse_error(what: impl Into<String>, hint: impl Into<String>) -> GfileError {
    GfileError::Parse {
        what: what.into(),
        hint: hint.into(),
    }
}

fn parse_hint() -> &'static str {
    "Page structure may have changed; rerun with --dump-page and -vv, then report the fixture."
}
