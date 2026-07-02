# NOTICE

This project is a Rust rewrite derived from and substantially informed by the
GPL-3.0 `gfile` implementations:

- `Sraq-Zit/gfile`, the original repository.
- `fireattack/gfile`, the active fork and source of the PyPI package
  `gigafile`.

The implementation reference is commit
`4c45392d2cc99903b38653b34e1dd07706c9c65a`.

Protocol observations used by this rewrite, including endpoint paths, page
selectors, multipart field names, and upload/download flow, come from reading
the source code at that commit and from public web behavior.
