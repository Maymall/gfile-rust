# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic
Versioning.

## [Unreleased]

## [0.2.0] - 2026-07-03

### Added

- Implement single-file uploads with serial streaming multipart chunks.
- Add upload landing-page parsing and local fixture coverage.
- Add upload verification via returned download page `Content-Length`.
- Add upload JSON output, CLI snapshots, and upload error coverage for rejected uploads and verification failures.

## [0.1.0] - 2026-07-03

### Added

- Bootstrap the Rust package, CLI shell, GPL compliance files, and CI workflow.
- Implement single-file and matomete downloads with sequential execution.
- Add download keys via `--key`, `--password`, and `-k`.
- Add resumable `.part` downloads with `--no-resume`.
- Add final `--json` output and CLI snapshot coverage.
- Add parser fixtures for matomete, password-required, wrong-key, missing, expired, and blocked pages.
- Preserve real UTF-8 filenames from `Content-Disposition` when the HTML page masks the displayed name.
