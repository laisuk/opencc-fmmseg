# ruzstd decoder attribution

This directory contains a decoder-only adaptation of source code from
**ruzstd 0.9.0**, part of the `zstd-rs` project by Moritz Borcherding and
contributors.

Upstream: https://github.com/KillingSpark/zstd-rs
Version used: 0.9.0
License: MIT (see `LICENSE-RUZSTD`)

OpenCC adaptation:
- removed all encoder modules;
- removed `StreamingDecoder`;
- removed dictionary-building, fuzz, benchmark, and test-only encoder helpers;
- removed no_std abstraction because opencc-fmmseg uses std;
- adjusted internal `crate::...` paths for nesting under `crate::zstd`;
- exposes a small `decompress_into()` entry point around `FrameDecoder::decode_all()`.

The Zstandard *decoding dictionary* implementation remains because it is part
of FrameDecoder's supported frame machinery; the unrelated dictionary-builder
module is not included.
