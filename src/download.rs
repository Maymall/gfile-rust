// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use reqwest::header;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{info, warn};

use crate::{
    error::{BoxError, GfileError, IoOp},
    http,
    naming::{log_name_diagnostics, sanitize_server_filename},
    parser::download::{RemoteFile, parse_single_file_page},
    progress::ByteProgress,
    urlinfo::parse_download_url,
};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url: String,
    pub output: Option<PathBuf>,
    pub force: bool,
    pub timeout: Duration,
    pub retries: u32,
    pub user_agent: Option<String>,
    pub dump_page: Option<PathBuf>,
    pub quiet: bool,
    pub allow_any_host: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub bytes: u64,
    pub resumed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PartSidecar {
    version: u8,
    file_id: String,
    expected: Option<u64>,
    key_used: bool,
}

pub async fn download(options: DownloadOptions) -> Result<DownloadOutcome, GfileError> {
    let url_info = parse_download_url(&options.url, options.allow_any_host)?;
    let client = http::build_client(options.user_agent.as_deref())?;

    let page_response = http::get_with_retries(
        &client,
        &url_info.page_url,
        options.retries,
        "fetching page",
    )
    .await?;
    let page_bytes = page_response
        .bytes()
        .await
        .map_err(|source| network_error(source, "reading download page body"))?;

    if let Some(path) = &options.dump_page {
        fs::write(path, &page_bytes)
            .await
            .map_err(|source| io_error(source, path, IoOp::Write))?;
        eprintln!("Warning: dumped page may contain private filenames; do not share it publicly.");
    }

    let html = String::from_utf8_lossy(&page_bytes);
    let page = parse_single_file_page(&html, &url_info.file_id)?;
    let remote_file = page.files.first().ok_or_else(|| GfileError::Parse {
        what: "page contains no downloadable file".to_owned(),
        hint: "Page structure may have changed; rerun with --dump-page and -vv.".to_owned(),
    })?;

    let final_path = resolve_output_path(remote_file, options.output.as_deref()).await?;
    if final_path.exists() && !options.force {
        return Err(io_error(
            io::Error::new(io::ErrorKind::AlreadyExists, "target exists"),
            &final_path,
            IoOp::Create,
        ));
    }

    let sanitized_name = sanitize_server_filename(&remote_file.raw_name, &remote_file.file_id);
    log_name_diagnostics(&remote_file.raw_name, &sanitized_name, &final_path);

    download_file_with_retries(
        &client,
        &url_info.download_url(),
        remote_file,
        &final_path,
        &options,
    )
    .await
}

async fn download_file_with_retries(
    client: &reqwest::Client,
    download_url: &str,
    remote_file: &RemoteFile,
    final_path: &Path,
    options: &DownloadOptions,
) -> Result<DownloadOutcome, GfileError> {
    let mut attempt = 0;
    loop {
        match try_download_file(client, download_url, remote_file, final_path, options).await {
            Ok(outcome) => return Ok(outcome),
            Err(error) if http::is_retryable(&error) && attempt < options.retries => {
                warn!(
                    "retrying file download after error: {}",
                    error.user_message()
                );
                tokio::time::sleep(http::retry_delay(attempt)).await;
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

async fn try_download_file(
    client: &reqwest::Client,
    download_url: &str,
    remote_file: &RemoteFile,
    final_path: &Path,
    options: &DownloadOptions,
) -> Result<DownloadOutcome, GfileError> {
    let response = http::get_once(
        client,
        download_url,
        "starting file download",
        Some(options.timeout),
    )
    .await?;

    if !response.status().is_success() {
        return Err(http::status_error(response.status(), download_url));
    }

    if is_html_content_type(response.headers()) {
        return Err(html_response_error(
            "download response content-type is HTML",
        ));
    }

    let expected = response.content_length();
    if expected.is_none() {
        warn!("download response has no Content-Length; exact size check is disabled");
    }
    if let (Some(display_size_text), Some(approx), Some(content_length)) = (
        remote_file.display_size.as_deref(),
        remote_file.approx_bytes,
        expected,
    ) {
        let tolerance = (approx / 10).max(1024);
        if approx.abs_diff(content_length) > tolerance {
            warn!(
                "display size {} differs from Content-Length {} by more than tolerance",
                display_size_text, content_length
            );
        }
    }

    let (part_path, sidecar_path) = part_paths(final_path)?;
    if part_path.exists() {
        warn!(
            "existing .part file found at {}; M1 has no resume support, starting from zero",
            part_path.display()
        );
    }

    let mut response = response;
    let first_chunk = match tokio::time::timeout(options.timeout, response.chunk()).await {
        Ok(Ok(Some(chunk))) => Some(chunk.to_vec()),
        Ok(Err(source)) => {
            return Err(network_error(source, "reading first download chunk"));
        }
        Ok(Ok(None)) => None,
        Err(_) => return Err(timeout_network_error("reading first download chunk")),
    };

    if first_chunk
        .as_ref()
        .is_some_and(|chunk| looks_like_html(chunk.as_ref()))
    {
        return Err(html_response_error(
            "download response body looks like HTML",
        ));
    }

    let sidecar = PartSidecar {
        version: 1,
        file_id: remote_file.file_id.clone(),
        expected,
        key_used: false,
    };
    let sidecar_bytes = serde_json::to_vec(&sidecar).map_err(|source| GfileError::Parse {
        what: format!("failed to serialize sidecar: {source}"),
        hint: "This is an internal state error; please report it.".to_owned(),
    })?;
    fs::write(&sidecar_path, sidecar_bytes)
        .await
        .map_err(|source| io_error(source, &sidecar_path, IoOp::Write))?;

    let file = File::create(&part_path)
        .await
        .map_err(|source| io_error(source, &part_path, IoOp::Create))?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    let progress = ByteProgress::new(expected, options.quiet, &remote_file.raw_name);
    let mut actual = 0_u64;

    if let Some(chunk) = first_chunk {
        writer
            .write_all(&chunk)
            .await
            .map_err(|source| io_error(source, &part_path, IoOp::Write))?;
        actual += chunk.len() as u64;
        progress.inc(chunk.len() as u64);
    }

    loop {
        let chunk = match next_chunk(&mut response, options.timeout).await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(ChunkReadError::Timeout) => {
                return Err(timeout_network_error("reading download chunk"));
            }
            Err(ChunkReadError::Http(source)) if expected.is_some() && source.is_decode() => {
                return Err(GfileError::SizeMismatch {
                    expected: expected.unwrap(),
                    actual,
                });
            }
            Err(ChunkReadError::Http(source)) => {
                return Err(network_error(source, "reading download chunk"));
            }
        };
        writer
            .write_all(&chunk)
            .await
            .map_err(|source| io_error(source, &part_path, IoOp::Write))?;
        actual += chunk.len() as u64;
        progress.inc(chunk.len() as u64);
    }
    progress.finish();

    if let Some(expected) = expected {
        if actual != expected {
            return Err(GfileError::SizeMismatch { expected, actual });
        }
    }

    writer
        .flush()
        .await
        .map_err(|source| io_error(source, &part_path, IoOp::Write))?;
    let file = writer.into_inner();
    file.sync_all()
        .await
        .map_err(|source| io_error(source, &part_path, IoOp::Write))?;

    if final_path.exists() && options.force {
        fs::remove_file(final_path)
            .await
            .map_err(|source| io_error(source, final_path, IoOp::Rename))?;
    }

    fs::remove_file(&sidecar_path)
        .await
        .map_err(|source| io_error(source, &sidecar_path, IoOp::Write))?;
    fs::rename(&part_path, final_path)
        .await
        .map_err(|source| io_error(source, final_path, IoOp::Rename))?;

    info!("downloaded {} bytes to {}", actual, final_path.display());

    Ok(DownloadOutcome {
        path: final_path.to_owned(),
        bytes: actual,
        resumed: false,
    })
}

async fn next_chunk(
    response: &mut reqwest::Response,
    timeout: Duration,
) -> Result<Option<Vec<u8>>, ChunkReadError> {
    match tokio::time::timeout(timeout, response.chunk()).await {
        Ok(Ok(Some(chunk))) => Ok(Some(chunk.to_vec())),
        Ok(Err(source)) => Err(ChunkReadError::Http(source)),
        Ok(Ok(None)) => Ok(None),
        Err(_) => Err(ChunkReadError::Timeout),
    }
}

enum ChunkReadError {
    Http(reqwest::Error),
    Timeout,
}

async fn resolve_output_path(
    remote_file: &RemoteFile,
    output: Option<&Path>,
) -> Result<PathBuf, GfileError> {
    match output {
        Some(path) if path.exists() && path.is_dir() => {
            let name = sanitize_server_filename(&remote_file.raw_name, &remote_file.file_id);
            Ok(path.join(name))
        }
        Some(path) => Ok(path.to_owned()),
        None => {
            let name = sanitize_server_filename(&remote_file.raw_name, &remote_file.file_id);
            std::env::current_dir()
                .map(|cwd| cwd.join(name))
                .map_err(|source| io_error(source, Path::new("."), IoOp::Metadata))
        }
    }
}

fn part_paths(final_path: &Path) -> Result<(PathBuf, PathBuf), GfileError> {
    let file_name = final_path.file_name().ok_or_else(|| {
        io_error(
            io::Error::new(io::ErrorKind::InvalidInput, "target path has no filename"),
            final_path,
            IoOp::Create,
        )
    })?;
    let part_name = format!("{}.part", file_name.to_string_lossy());
    let sidecar_name = format!("{part_name}.json");

    let mut part = final_path.to_owned();
    part.set_file_name(part_name);
    let mut sidecar = final_path.to_owned();
    sidecar.set_file_name(sidecar_name);
    Ok((part, sidecar))
}

fn is_html_content_type(headers: &header::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/html"))
}

fn looks_like_html(bytes: &[u8]) -> bool {
    let prefix_len = bytes.len().min(512);
    let prefix = String::from_utf8_lossy(&bytes[..prefix_len]).to_ascii_lowercase();
    let trimmed = prefix.trim_start();
    trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || trimmed.contains("<html")
        || trimmed.contains("<body")
}

fn html_response_error(what: &str) -> GfileError {
    GfileError::Parse {
        what: what.to_owned(),
        hint: "The server returned an HTML page instead of a file; rerun with --dump-page and -vv for diagnostics.".to_owned(),
    }
}

fn network_error(source: reqwest::Error, context: &str) -> GfileError {
    GfileError::Network {
        source: boxed(source),
        context: context.to_owned(),
    }
}

fn timeout_network_error(context: &str) -> GfileError {
    GfileError::Network {
        source: boxed(io::Error::new(io::ErrorKind::TimedOut, "stream timed out")),
        context: context.to_owned(),
    }
}

fn io_error(source: io::Error, path: &Path, op: IoOp) -> GfileError {
    GfileError::Io {
        source,
        path: path.to_owned(),
        op,
    }
}

fn boxed(error: impl std::error::Error + Send + Sync + 'static) -> BoxError {
    Box::new(error)
}
