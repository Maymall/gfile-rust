// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use gfile_rust::{
    download::{DownloadOptions, download},
    error::GfileError,
};
use tempfile::TempDir;
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{method, path, query_param},
};

const FILE_ID: &str = "0123abcd-000000example";

#[tokio::test]
async fn download_single_success_writes_final_and_cleans_part_files() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    let body = binary_body(10 * 1024);
    mount_file(&server, 200, body.clone(), Some(body.len()), None).await;
    let temp = TempDir::new().unwrap();

    let outcome = download(options(&server, &temp, 3)).await.unwrap();

    assert_eq!(outcome.bytes, body.len() as u64);
    assert_eq!(std::fs::read(&outcome.path).unwrap(), body);
    assert!(
        !outcome
            .path
            .with_file_name("example file.bin.part")
            .exists()
    );
    assert!(
        !outcome
            .path
            .with_file_name("example file.bin.part.json")
            .exists()
    );
}

#[tokio::test]
async fn download_single_japanese_name_preserves_filename_bytes() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_japanese.html")).await;
    let body = binary_body(1024);
    mount_file(&server, 200, body, Some(1024), None).await;
    let temp = TempDir::new().unwrap();

    let outcome = download(options(&server, &temp, 3)).await.unwrap();

    assert_eq!(
        outcome
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .as_bytes(),
        "テスト資料_2026.zip".as_bytes()
    );
}

#[tokio::test]
async fn download_size_mismatch_keeps_part_file() {
    let body = binary_body(10 * 1024);
    let server_uri = start_raw_mismatch_server(
        include_str!("fixtures/single_basic.html")
            .as_bytes()
            .to_vec(),
        body,
        12 * 1024,
    );
    let temp = TempDir::new().unwrap();
    let opts = DownloadOptions {
        url: format!("{server_uri}/{FILE_ID}"),
        output: Some(temp.path().to_owned()),
        force: false,
        timeout: Duration::from_secs(60),
        retries: 0,
        user_agent: None,
        dump_page: None,
        quiet: true,
        allow_any_host: true,
    };

    let error = download(opts).await.expect_err("size mismatch should fail");

    match error {
        GfileError::SizeMismatch { expected, actual } => {
            assert_eq!(expected, 12 * 1024);
            assert_eq!(actual, 10 * 1024);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(temp.path().join("example file.bin.part").exists());
    assert!(temp.path().join("example file.bin.part.json").exists());
    assert!(!temp.path().join("example file.bin").exists());
}

#[tokio::test]
async fn download_html_response_is_not_written_to_disk() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    Mock::given(method("GET"))
        .and(path("/download.php"))
        .and(query_param("file", FILE_ID))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "<!doctype html><html><body>not a file</body></html>",
            "text/html",
        ))
        .mount(&server)
        .await;
    let temp = TempDir::new().unwrap();

    let error = download(options(&server, &temp, 0))
        .await
        .expect_err("HTML response should fail");

    assert!(matches!(error, GfileError::Parse { .. }));
    assert!(!temp.path().join("example file.bin").exists());
    assert!(!temp.path().join("example file.bin.part").exists());
}

#[tokio::test]
async fn download_retries_503_then_succeeds() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    let body = binary_body(4096);
    let counter = Arc::new(AtomicUsize::new(0));
    let responder_counter = Arc::clone(&counter);
    Mock::given(method("GET"))
        .and(path("/download.php"))
        .and(query_param("file", FILE_ID))
        .respond_with(move |_request: &Request| {
            let attempt = responder_counter.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200)
                    .insert_header("Content-Length", body.len().to_string())
                    .set_body_bytes(body.clone())
            }
        })
        .mount(&server)
        .await;
    let temp = TempDir::new().unwrap();

    let outcome = download(options(&server, &temp, 3)).await.unwrap();

    assert_eq!(outcome.bytes, 4096);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn download_retry_exhaustion_returns_http_status() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    Mock::given(method("GET"))
        .and(path("/download.php"))
        .and(query_param("file", FILE_ID))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let temp = TempDir::new().unwrap();

    let error = download(options(&server, &temp, 1))
        .await
        .expect_err("503 should fail after retries");

    assert!(matches!(error, GfileError::HttpStatus { status: 503, .. }));
    let requests = server.received_requests().await.unwrap();
    let download_requests = requests
        .iter()
        .filter(|request| request.url.path() == "/download.php")
        .count();
    assert_eq!(download_requests, 2);
}

#[tokio::test]
async fn download_without_content_length_succeeds() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    let body = binary_body(2048);
    mount_file(&server, 200, body, None, None).await;
    let temp = TempDir::new().unwrap();

    let outcome = download(options(&server, &temp, 0)).await.unwrap();

    assert_eq!(outcome.bytes, 2048);
    assert!(outcome.path.exists());
}

#[tokio::test]
async fn download_stall_timeout_retries_and_succeeds() {
    let server = MockServer::start().await;
    mount_page(&server, include_str!("fixtures/single_basic.html")).await;
    let body = binary_body(1024);
    let counter = Arc::new(AtomicUsize::new(0));
    let responder_counter = Arc::clone(&counter);
    Mock::given(method("GET"))
        .and(path("/download.php"))
        .and(query_param("file", FILE_ID))
        .respond_with(move |_request: &Request| {
            let attempt = responder_counter.fetch_add(1, Ordering::SeqCst);
            let response = ResponseTemplate::new(200)
                .insert_header("Content-Length", body.len().to_string())
                .set_body_bytes(body.clone());
            if attempt == 0 {
                response.set_delay(Duration::from_secs(2))
            } else {
                response
            }
        })
        .mount(&server)
        .await;
    let temp = TempDir::new().unwrap();
    let mut opts = options(&server, &temp, 1);
    opts.timeout = Duration::from_secs(1);

    let outcome = download(opts).await.unwrap();

    assert_eq!(outcome.bytes, 1024);
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

async fn mount_page(server: &MockServer, body: &'static str) {
    Mock::given(method("GET"))
        .and(path(format!("/{FILE_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/html"))
        .mount(server)
        .await;
}

async fn mount_file(
    server: &MockServer,
    status: u16,
    body: Vec<u8>,
    content_length: Option<usize>,
    content_type: Option<&str>,
) {
    let mut response = ResponseTemplate::new(status).set_body_bytes(body);
    if let Some(content_length) = content_length {
        response = response.insert_header("Content-Length", content_length.to_string());
    }
    if let Some(content_type) = content_type {
        response = response.insert_header("Content-Type", content_type);
    }
    Mock::given(method("GET"))
        .and(path("/download.php"))
        .and(query_param("file", FILE_ID))
        .respond_with(response)
        .mount(server)
        .await;
}

fn options(server: &MockServer, temp: &TempDir, retries: u32) -> DownloadOptions {
    DownloadOptions {
        url: format!("{}/{FILE_ID}", server.uri()),
        output: Some(temp.path().to_owned()),
        force: false,
        timeout: Duration::from_secs(60),
        retries,
        user_agent: None,
        dump_page: None,
        quiet: true,
        allow_any_host: true,
    }
}

fn binary_body(size: usize) -> Vec<u8> {
    (0..size).map(|index| (index % 251) as u8).collect()
}

fn start_raw_mismatch_server(
    page_body: Vec<u8>,
    file_body: Vec<u8>,
    declared_len: usize,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            if request.starts_with(&format!("GET /{FILE_ID} ")) {
                write_response(&mut stream, "text/html", page_body.len(), &page_body);
            } else {
                write_response(
                    &mut stream,
                    "application/octet-stream",
                    declared_len,
                    &file_body,
                );
            }
        }
    });
    format!("http://{addr}")
}

fn write_response(stream: &mut std::net::TcpStream, content_type: &str, len: usize, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}
