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
pub(crate) fn decompress_into(input: &[u8], output: &mut [u8]) -> Result<usize, FrameDecoderError> {
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
/// Decompresses Zstandard-compressed data into a newly allocated byte vector.
///
/// The frame is decoded incrementally, so the uncompressed size does not need
/// to be known in advance. This allows decoding older or streaming-generated
/// Zstandard frames that do not contain a frame content size (FCS).
///
/// If the frame declares an FCS of at most 64 MiB, it is used only as a
/// preallocation hint for the output vector. Larger or unavailable sizes are
/// ignored, and the output vector grows as needed. The FCS therefore does not
/// affect whether a valid frame can be decoded.
///
/// During incremental decoding, the decoder retains the history window required
/// for Zstandard backreferences and collects output as it becomes available.
pub(crate) fn decompress(input: &[u8]) -> Result<Vec<u8>, FrameDecoderError> {
    let mut decoder = FrameDecoder::new();
    let mut source = input;

    decoder.init(&mut source)?;

    const MAX_PREALLOC_SIZE: u64 = 64 * 1024 * 1024;

    let mut output = decoder
        .content_size()
        .filter(|&size| size <= MAX_PREALLOC_SIZE)
        .and_then(|size| usize::try_from(size).ok())
        .map(Vec::with_capacity)
        .unwrap_or_default();

    while !decoder.is_finished() {
        decoder.decode_blocks(&mut source, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;

        if let Some(chunk) = decoder.collect() {
            output.extend_from_slice(&chunk);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DictionaryMaxlength;

    #[test]
    fn decompress_exact_matches_original() {
        let compressed = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let decoded =
            decompress_exact(compressed, expected.len()).expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[test]
    fn decompress_unknown_size_matches_original() {
        let compressed = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let (header, _) = decoding::frame::read_frame_header(&compressed[..]).unwrap();
        assert_eq!(header.frame_content_size(), 0);

        let decoded = decompress(compressed).expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[cfg(all(test, feature = "dictionary-build"))]
    #[test]
    fn one_shot_zstd_has_content_size() {
        let input = b"OpenCC FCS regression test";

        let compressed = zstd::bulk::compress(input, 3).expect("compression failed");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("frame initialization failed");

        assert_eq!(decoder.content_size(), Some(input.len() as u64));
    }
}

#[cfg(test)]
mod regression_tests;
