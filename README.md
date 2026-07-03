# gfile-rust

[![CI](https://github.com/Maymall/gfile-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Maymall/gfile-rust/actions/workflows/ci.yml)
[![Release](https://github.com/Maymall/gfile-rust/actions/workflows/release.yml/badge.svg)](https://github.com/Maymall/gfile-rust/actions/workflows/release.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue.svg)](LICENSE)

`gfile-rust` is a Rust command line tool for automating public GigaFile web
upload and download flows. Download support covers single-file and matomete
pages; upload support covers single-file uploads.

## Install

### Download a Release

Download the archive for your platform from
[GitHub Releases](https://github.com/Maymall/gfile-rust/releases), then verify
it with `SHA256SUMS`:

```bash
sha256sum -c SHA256SUMS
```

Archives contain the `gfile` binary plus `LICENSE`, `NOTICE`, `NOTICE.md`, and
`README.md`.

| Platform | Target | Archive |
|---|---|---|
| Linux x86_64 glibc | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux x86_64 musl | `x86_64-unknown-linux-musl` | `.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

### Install from Source

```bash
cargo install --git https://github.com/Maymall/gfile-rust --tag v0.3.0
```

## Upload Usage

Upload a local file and print the public download page URL:

```bash
gfile upload ./example-file.bin
```

The default lifetime is 100 days. GigaFile-supported lifetimes can be selected
explicitly:

```bash
gfile upload ./example-file.bin --lifetime 7
```

Uploads are split into serial multipart chunks. The default chunk size is
100MiB; values from 1MiB through 1GiB are accepted:

```bash
gfile upload ./example-file.bin --chunk-size 50M
```

`--timeout` is an idle timeout. During upload, it is measured from the last body
stream progress or response completion activity, not from the total chunk
duration.

After upload, `gfile-rust` verifies the returned download page by checking the
remote `Content-Length`. Use `--no-verify` only when the server cannot expose a
reliable length and you have another way to validate the result.

For scripts, upload also supports final JSON output:

```bash
gfile upload --json ./example-file.bin
```

## Download Usage

Download a public file page into the current directory:

```bash
gfile download https://23.gigafile.nu/0123abcd-000000example
```

Choose an output directory or, for a single-file page, an explicit filename:

```bash
gfile download https://23.gigafile.nu/0123abcd-000000example -o ./downloads
gfile download https://23.gigafile.nu/0123abcd-000000example -o "./example file.bin"
```

Download with a key:

```bash
gfile download https://23.gigafile.nu/0123abcd-000000example --key EXAMPLE-KEY-0000
```

If a page requires a key and `--key` is not provided, an interactive terminal
will prompt once without echoing input. Non-interactive runs exit with code 15.

Resume is enabled by default when a matching `.part` and `.part.json` sidecar
exist. Use `--no-resume` to ignore partial state and start from zero.

For scripts, use `--json` to print one final JSON object to stdout and suppress
progress output:

```bash
gfile download --json https://23.gigafile.nu/0123abcd-000000example
```

## Behavior

- Downloads retry retryable network failures and HTTP 5xx responses.
- Download `--timeout` is a per-read stall timeout.
- Uploads retry retryable network failures and HTTP 5xx chunk responses.
- Upload `--timeout` is an idle timeout while sending a chunk or waiting for the
  chunk response.
- Matomete pages are downloaded sequentially. If one file fails, later files are
  still attempted and the final process exit code is the first failure code.
- Uploads are intentionally serial and stream chunks from disk.
- When GigaFile's page masks the displayed filename, `gfile-rust` prefers the
  `Content-Disposition` filename from the actual file response, including UTF-8
  `filename*=` values.

## From Python gfile

| Python gfile | gfile-rust | Notes |
|---|---|---|
| `gfile upload FILE` | `gfile upload FILE` | Uploads are intentionally serial and stream chunks from disk. |
| fixed upload lifetime | `--lifetime <DAYS>` | Accepted values: 3, 5, 7, 14, 30, 60, 100. |
| upload progress | upload progress | Progress advances while streaming each chunk and is reset on retry until the chunk is confirmed. |
| upload timeout | `--timeout <SECONDS>` | Idle timeout, not total chunk duration. |
| `gfile download URL` | `gfile download URL` | Same basic download shape. |
| `--key` / `--password` | `--key` / `--password` / `-k` | Password value is sent as `dlkey`. |
| output filename | `-o <PATH>` | For matomete, `-o` must be an existing directory. |
| built-in sequential download | built-in sequential download | Matomete files are intentionally not downloaded in parallel. |
| threaded upload | not implemented | This build avoids high-concurrency upload behavior. |
| `--aria2` | not implemented | Multi-connection aria2 integration is planned only as a backlog item. |
| JSON output | `--json` | Rust version provides a stable final JSON object. |

## Exit Codes

| Code | Error | Meaning |
|---:|---|---|
| 2 | `usage` | Invalid CLI arguments or unsupported option value. |
| 10 | `invalid_url` | URL is not a supported public download page URL. |
| 11 | `network` | Network request, timeout, or retry exhaustion failure. |
| 12 | `http_status` | Unexpected non-retryable HTTP status while downloading. |
| 13 | `parse` | Required page data could not be parsed. |
| 14 | `not_found_or_expired` | File page reports missing or expired content. |
| 15 | `key_required` | Download key is required but unavailable. |
| 16 | `password_wrong` | Download key was rejected. |
| 17 | `size_mismatch` | Downloaded size did not match the expected size. |
| 18 | `io` | Local filesystem error. |
| 19 | `upload_rejected` | Upload endpoint rejected the upload. |
| 20 | `verify_failed` | Upload verification found a size mismatch. |

## Security

Download keys are never written to the resume sidecar; the sidecar stores only
whether a key was used. Cookies are kept in memory and are not persisted.

Passing `--key EXAMPLE-KEY-0000` can expose the value through shell history or
process listings such as `ps`. Prefer the interactive prompt when that matters.
Do not publish `--dump-page` output without reviewing it; it may contain private
filenames or page details.

## GPL Compliance

- License: GPL-3.0-only; see [LICENSE](LICENSE).
- Attribution: this rewrite is derived from and substantially informed by
  GPL-3.0 `Sraq-Zit/gfile` and `fireattack/gfile`; see [NOTICE.md](NOTICE.md).
- Binary release archives include `LICENSE`, `NOTICE`, `NOTICE.md`, and
  `README.md`.
- Corresponding source for a release is the repository tag with the same name,
  for example `v0.3.0`.
- No additional license restrictions are imposed by the release packaging.

## Disclaimer

This is an unofficial tool. Users are responsible for complying with GigaFile's
official terms and acceptable-use rules.

## Behavior Boundaries

- No password guessing, dictionary attacks, link scanning, or enumeration.
- No high-concurrency stress or load-testing mode.
- No bypass of download pages, advertising, membership restrictions, or other
  service controls.
- No persistence of cookies, passwords, tokens, or download keys to disk.
- No third-party email notification features.
- No browser impersonation for bypass purposes.
