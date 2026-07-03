# rgfile

[![CI](https://github.com/Maymall/gigafile-rust-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Maymall/gigafile-rust-cli/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/rgfile.svg)](https://crates.io/crates/rgfile)
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue.svg)](LICENSE)

`rgfile` is a fast, robust command-line client for [GigaFile.nu](https://gigafile.nu):
upload and download files straight from the terminal.

## Features

- Downloads single-file and multi-file (matomete) pages, with `--key` for password-protected links
- Resumable downloads: interrupted transfers continue from where they stopped, completion is atomic and size-verified
- Correct filenames: decoded from `Content-Disposition` (RFC 5987), so UTF-8 / Japanese names survive intact
- Streaming uploads with constant memory (~10 MiB peak, independent of chunk size) and per-chunk retry
- Upload results include the download URL, delete key, and estimated expiry; lifetime selectable (3–100 days)
- `rgfile info <url>` inspects a page without downloading
- Optional TOML config and opt-in local history (`rgfile history list`)
- `--json` output and stable exit codes for scripting
- Static musl Linux binary, plus macOS (arm64/Intel) and Windows builds

## Install

One-liner (Linux / macOS; verifies SHA-256, installs to `~/.local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/Maymall/gigafile-rust-cli/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Maymall/gigafile-rust-cli/main/install.ps1 | iex
```

Other options:

```bash
cargo install rgfile                 # crates.io, needs Rust 1.85+
brew install Maymall/tap/rgfile      # Homebrew (macOS / Linux)
```

Debian / Ubuntu: download `rgfile_<version>_amd64.deb` from the
[latest release](https://github.com/Maymall/gigafile-rust-cli/releases/latest)
and `sudo apt install ./rgfile_<version>_amd64.deb`. Release archives for all
platforms (with `SHA256SUMS`) are on the same page.

### Upgrade

Rerun the install one-liner, or `cargo install rgfile`, or
`brew upgrade rgfile` — whichever you installed with.

## Usage

```bash
# Download
rgfile download https://23.gigafile.nu/0123abcd-000000example
rgfile download https://23.gigafile.nu/0123abcd-000000example -o ./downloads
rgfile download https://23.gigafile.nu/0123abcd-000000example --key EXAMPLE-KEY-0000
rgfile download --json https://23.gigafile.nu/0123abcd-000000example

# Upload (prints the download URL and the delete key)
rgfile upload ./example-file.bin
rgfile upload ./example-file.bin --lifetime 7
rgfile upload --json ./example-file.bin

# Inspect a page without downloading
rgfile info https://23.gigafile.nu/0123abcd-000000example
```

If a page needs a key and none is given, `rgfile` prompts on an interactive
terminal; non-interactive runs exit with code 15. See `rgfile <command> --help`
for all options (`--timeout`, `--retries`, `--no-resume`, `--chunk-size`, ...).

## Configuration

Optional TOML file at `~/.config/rgfile/config.toml` (Linux),
`~/Library/Application Support/rgfile/config.toml` (macOS), or
`%APPDATA%\rgfile\config.toml` (Windows). CLI options override config values;
`--config <path>` loads a specific file, `--no-config` skips loading.

```toml
[download]
dir = "/home/alice/Downloads"  # default output directory
threads = 1                    # connections per file, 1-16 (see note below)

[upload]
lifetime = 7                   # default lifetime in days: 3/5/7/14/30/60/100

[network]
timeout = 60                   # idle timeout in seconds
retries = 3

[history]
enabled = false                # opt-in local history
store_delete_keys = false      # keep upload delete keys in history (plaintext)
```

## History

Off by default. Enable with `history.enabled = true` (or `--history` for one
command). Records go to `~/.local/share/rgfile/history.jsonl` (platform
equivalent): timestamp, operation, URL, file names, bytes, result.

```bash
rgfile history list
rgfile history clear
```

Download passwords are never stored. Upload delete keys are stored only if you
opt in with `history.store_delete_keys = true`.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Success |
| 2 | Invalid CLI arguments or option value |
| 10 | Not a supported GigaFile URL |
| 11 | Network failure / timeout / retries exhausted |
| 12 | Unexpected HTTP status |
| 13 | Page could not be parsed |
| 14 | File not found or expired |
| 15 | Download key required |
| 16 | Download key rejected |
| 17 | Downloaded size mismatch |
| 18 | Local filesystem error |
| 19 | Upload rejected by the server |
| 20 | Upload verification failed |

## Notes

- Transfers use one connection by default. `--threads N` / `download.threads`
  enables experimental segmented downloads; GigaFile currently answers ranged
  requests with the full file, in which case rgfile automatically continues on
  a single connection.
- Uploads cannot resume across runs; a failed upload restarts from scratch.
- rgfile does not bypass GigaFile restrictions, guess passwords, or scrape links.

## License

GPL-3.0-only; see [LICENSE](LICENSE). Derived from the GPL-licensed Python
projects [`Sraq-Zit/gfile`](https://github.com/Sraq-Zit/gfile) and
[`fireattack/gfile`](https://github.com/fireattack/gfile) — see
[NOTICE.md](NOTICE.md). Corresponding source for a binary release is this
repository at the matching tag.
