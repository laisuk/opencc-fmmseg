//! Minimal decoder-only Zstandard implementation for OpenCC embedded resources.
//!
//! Derived from ruzstd 0.9.0 (MIT), by Moritz Borcherding and contributors.
//! Encoder, streaming wrapper, dictionary builder, fuzz/test helpers, and no_std
//! compatibility layers have intentionally been omitted.
//!
//! See `LICENSE-RUZSTD` and `NOTICE.md` in this directory.

macro_rules! vprintln {
    ($($x:expr),*) => {{ /* decoder tracing intentionally disabled */ }};
}

mod bit_io;
mod blocks;
mod common;
mod decoding;
mod fse;
mod huff0;
mod io;

pub(crate) use decoding::errors::FrameDecoderError;
pub(crate) use decoding::FrameDecoder;

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

pub(crate) fn decompress(
    input: &[u8],
) -> Result<Vec<u8>, FrameDecoderError> {
    let mut decoder = FrameDecoder::new();
    decoder.init(input)?;

    let expected_size = usize::try_from(decoder.content_size())
        .map_err(|_| FrameDecoderError::TargetTooSmall)?;

    let mut output = vec![0u8; expected_size];

    // decode_all() initializes again, unfortunately.
    let written = decoder.decode_all(input, &mut output)?;
    output.truncate(written);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_matches_original() {
        let compressed = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let decoded =
            decompress_exact(compressed, expected.len())
                .expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[test]
    fn inspect_zstd_content_size() {
        let compressed =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected =
            include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let mut decoder = FrameDecoder::new();
        decoder.init(compressed.as_slice()).unwrap();

        println!("frame content_size = {}", decoder.content_size());
        println!("actual CBOR size    = {}", expected.len());

        assert_eq!(decoder.content_size() as usize, expected.len());
    }
}