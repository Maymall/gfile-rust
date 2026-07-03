#!/bin/sh
# rgfile installer: downloads the latest release binary for this platform,
# verifies its SHA-256 against the release's SHA256SUMS, and installs it.
#
#   curl -fsSL https://raw.githubusercontent.com/Maymall/gigafile-rust-cli/main/install.sh | sh
#
# Override the install directory with RGFILE_INSTALL_DIR (default: ~/.local/bin).
set -eu

REPO="Maymall/gigafile-rust-cli"
INSTALL_DIR="${RGFILE_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
err() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || err "curl is required"
command -v tar >/dev/null 2>&1 || err "tar is required"

os=$(uname -s)
arch=$(uname -m)
case "$os" in
    Linux)
        case "$arch" in
            # The static musl build runs on any Linux regardless of libc.
            x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
            *) err "unsupported Linux architecture: $arch — download a release archive manually" ;;
        esac
        ;;
    Darwin)
        case "$arch" in
            arm64) target="aarch64-apple-darwin" ;;
            x86_64) target="x86_64-apple-darwin" ;;
            *) err "unsupported macOS architecture: $arch — download a release archive manually" ;;
        esac
        ;;
    *)
        err "unsupported OS: $os — on Windows use install.ps1, otherwise download a release archive manually"
        ;;
esac

# Resolve the latest tag from the releases/latest redirect (no API rate limit).
latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") ||
    err "cannot resolve the latest release"
tag=${latest_url##*/}
version=${tag#v}
case "$version" in
    [0-9]*) ;;
    *) err "cannot determine the latest version (got tag: $tag)" ;;
esac

asset="rgfile-$version-$target.tar.gz"
base="https://github.com/$REPO/releases/download/$tag"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Downloading rgfile $version for $target ..."
curl -fsSL -o "$tmp/$asset" "$base/$asset" || err "download failed: $base/$asset"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" || err "download failed: $base/SHA256SUMS"

# Verify the checksum; refuse to install anything that does not match.
expected_line=$(grep "[[:space:]]$asset\$" "$tmp/SHA256SUMS") || err "no checksum for $asset in SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$tmp" && printf '%s\n' "$expected_line" | sha256sum -c - >/dev/null) || err "checksum verification FAILED"
elif command -v shasum >/dev/null 2>&1; then
    (cd "$tmp" && printf '%s\n' "$expected_line" | shasum -a 256 -c - >/dev/null) || err "checksum verification FAILED"
else
    err "need sha256sum or shasum to verify the download"
fi
say "Checksum OK."

tar -xzf "$tmp/$asset" -C "$tmp"
[ -f "$tmp/rgfile-$version-$target/rgfile" ] || err "archive layout unexpected; aborting"
mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/rgfile-$version-$target/rgfile" "$INSTALL_DIR/rgfile"

say "Installed: $INSTALL_DIR/rgfile ($("$INSTALL_DIR/rgfile" --version))"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        say ""
        say "NOTE: $INSTALL_DIR is not in your PATH. Add this to your shell profile:"
        say "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac
