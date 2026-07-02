# gfile-rust

`gfile-rust` is a Rust command line tool for automating public GigaFile web
upload and download flows. The project is currently in the M0 bootstrap stage;
transfer commands are defined but not implemented yet.

## Usage

TODO: fill in stable download and upload usage after the corresponding
milestones land.

## License

This project is licensed under GPL-3.0-only. See [LICENSE](LICENSE).

## Attribution

This project is a Rust rewrite derived from and substantially informed by
`Sraq-Zit/gfile` and `fireattack/gfile`, both GPL-3.0 projects. The pinned
reference commit is `4c45392d2cc99903b38653b34e1dd07706c9c65a`.

See [NOTICE.md](NOTICE.md) for details.

## Disclaimer

This is an unofficial tool. Users are responsible for complying with GigaFile's
official terms and acceptable-use rules, including
https://gigafile.nu/privacy.php.

## Behavior Boundaries

- No password guessing, dictionary attacks, link scanning, or enumeration.
- No high-concurrency stress or load-testing mode.
- No bypass of download pages, advertising, membership restrictions, or other
  service controls.
- No persistence of cookies, passwords, tokens, or download keys to disk.
- No third-party email notification features.
- No browser impersonation for bypass purposes.
