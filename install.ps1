# rgfile installer for Windows: downloads the latest release, verifies its
# SHA-256 against the release's SHA256SUMS, installs it, and adds it to PATH.
#
#   irm https://raw.githubusercontent.com/Maymall/gigafile-rust-cli/main/install.ps1 | iex
#
# Override the install directory with $env:RGFILE_INSTALL_DIR.
$ErrorActionPreference = "Stop"

$repo = "Maymall/gigafile-rust-cli"
$installDir = if ($env:RGFILE_INSTALL_DIR) { $env:RGFILE_INSTALL_DIR } else { "$env:LOCALAPPDATA\Programs\rgfile" }

$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$tag = $release.tag_name
$version = $tag.TrimStart("v")
$asset = "rgfile-$version-x86_64-pc-windows-msvc.zip"
$base = "https://github.com/$repo/releases/download/$tag"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "rgfile-install-$([System.Guid]::NewGuid())"
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Write-Host "Downloading rgfile $version for Windows x86_64 ..."
    Invoke-WebRequest -Uri "$base/$asset" -OutFile "$tmp\$asset"
    Invoke-WebRequest -Uri "$base/SHA256SUMS" -OutFile "$tmp\SHA256SUMS"

    # Verify the checksum; refuse to install anything that does not match.
    $sumLine = (Get-Content "$tmp\SHA256SUMS") | Where-Object { $_ -match [regex]::Escape($asset) }
    if (-not $sumLine) { throw "no checksum for $asset in SHA256SUMS" }
    $expected = ($sumLine -split '\s+')[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 "$tmp\$asset").Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum verification FAILED" }
    Write-Host "Checksum OK."

    Expand-Archive -Path "$tmp\$asset" -DestinationPath $tmp -Force
    $binary = "$tmp\rgfile-$version-x86_64-pc-windows-msvc\rgfile.exe"
    if (-not (Test-Path $binary)) { throw "archive layout unexpected; aborting" }
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item $binary "$installDir\rgfile.exe" -Force

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($userPath -split ";") -notcontains $installDir) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
        Write-Host "Added $installDir to your user PATH (restart the terminal to pick it up)."
    }
    Write-Host "Installed: $installDir\rgfile.exe ($(& "$installDir\rgfile.exe" --version))"
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
