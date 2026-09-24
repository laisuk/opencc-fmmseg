# OpenCC decoder-only Zstd module (experiment)

This is a decoder-only adaptation of **ruzstd 0.9.0** for the `opencc-fmmseg`
`zstd` experiment branch.

## Install manually

Copy this entire `zstd/` directory under the crate's `src/` directory and add:

```rust
mod zstd;
```

to the appropriate crate root/module tree.

The internal paths currently assume the module is exactly `crate::zstd`.

## Intended entry points

```rust
let mut raw = vec![0u8; EXPECTED_UNCOMPRESSED_SIZE];
let written = zstd::decompress_into(compressed, &mut raw)?;
raw.truncate(written);
```

or:

```rust
let raw = zstd::decompress_exact(compressed, EXPECTED_UNCOMPRESSED_SIZE)?;
```

`decompress_into()` is the preferred fast path: one caller-owned allocation,
no streaming wrapper, and no encoder code.

## Important experimental notes

* This bundle intentionally removes `StreamingDecoder`, all encoders, no_std
  support, dictionary building, fuzzing, benchmarks, and upstream tests.
* Zstd *decoding dictionary* support remains because it is integrated with
  `FrameDecoder`; it can be trimmed later after OpenCC regression testing.
* Upstream checksum hashing is feature-gated by `#[cfg(feature = "hash")]`.
  This bundle does not add `twox-hash`; therefore checksum calculation is not
  enabled unless you deliberately wire that feature/dependency into the host
  crate. Frame checksum bytes are still parsed as part of the frame format.
* I could structurally inspect and trim the source in this environment, but
  `rustc` is not installed here, so perform the first compile on the `zstd`
  branch before further pruning.
* Keep `LICENSE-RUZSTD` and `NOTICE.md` with redistributed source.
