//! Minimal decoder-only Zstandard implementation for OpenCC embedded resources.
//!
//! Derived from ruzstd 0.9.0 (MIT), by Moritz Borcherding and contributors.
//! Encoder, streaming wrapper, dictionary builder, fuzz/test helpers, and no_std
//! compatibility layers have intentionally been omitted.
//!
//! See `LICENSE-RUZSTD` and `NOTICE.md` in this directory.

mod bit_io;
mod blocks;
mod common;
mod decoding;
mod fse;
mod huff0;

pub(crate) use decoding::errors::FrameDecoderError;
use decoding::{BlockDecodingStrategy, FrameDecoder};

/// Decompress Zstandard data into a caller-provided output buffer.
///
/// This is the allocation-free fast path intended for embedded OpenCC resources
/// whose uncompressed size is known by the caller.
pub(crate) fn decompress_into(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, FrameDecoderError> {
    let mut decoder = FrameDecoder::new();
    decoder.decode_all(input, output)
}


/// Decompress into a newly allocated vector of exactly `expected_size` bytes.
///
/// Prefer this for embedded resources when their uncompressed size is available
/// as build-time metadata. A size mismatch is reported by `FrameDecoder` rather
/// than silently growing the allocation.
pub(crate) fn decompress_exact(
    input: &[u8],
    expected_size: usize,
) -> Result<Vec<u8>, FrameDecoderError> {
    let mut output = vec![0u8; expected_size];
    let written = decompress_into(input, &mut output)?;
    output.truncate(written);
    Ok(output)
}

/// Decompress Zstandard data without requiring a known uncompressed size.
///
/// This path decodes incrementally and collects output while preserving the
/// history window required for Zstandard backreferences. It is suitable for
/// external or generated frames that do not declare a frame content size.
pub(crate) fn decompress(
    input: &[u8],
) -> Result<Vec<u8>, FrameDecoderError> {
    let mut decoder = FrameDecoder::new();
    let mut source = input;

    // init() consumes the frame header from `source`.
    decoder.init(&mut source)?;

    let mut output = Vec::new();

    while !decoder.is_finished() {
        decoder.decode_blocks(
            &mut source,
            BlockDecodingStrategy::UptoBytes(1024 * 1024),
        )?;

        if let Some(chunk) = decoder.collect() {
            output.extend_from_slice(&chunk);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_exact_matches_original() {
        let compressed =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let decoded =
            decompress_exact(compressed, expected.len())
                .expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[test]
    fn decompress_unknown_size_matches_original() {
        let compressed =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let (header, _) = decoding::frame::read_frame_header(&compressed[..]).unwrap();
        assert_eq!(header.frame_content_size(), 0);

        let decoded =
            decompress(compressed)
                .expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }
}

#[cfg(test)]
mod regression_tests;
