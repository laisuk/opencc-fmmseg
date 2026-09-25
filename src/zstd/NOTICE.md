# ruzstd decoder attribution

This directory contains a decoder-only adaptation of source code from
**ruzstd 0.9.0**, part of the `zstd-rs` project by Moritz Borcherding and
contributors.

Upstream: https://github.com/KillingSpark/zstd-rs
Version used: 0.9.0
License: MIT (see `LICENSE-RUZSTD`)

OpenCC adaptation:
- removed encoders, StreamingDecoder, dictionary building and decoding-dictionary APIs;
- removed no_std, writer and unused upstream compatibility APIs;
- removed unused alternative ring-buffer implementations, retaining the active algorithms;
- adjusted internal paths for nesting under `crate::zstd`;
- retained crate-private `decompress()`, `decompress_into()`, and `decompress_exact()`.

See README.md for retained behavior and the dependency audit.
