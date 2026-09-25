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

const MAX_FCS_PREALLOC_SIZE: u64 = 64 * 1024 * 1024;

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

    let mut output = decoder
        .content_size()
        .filter(|&size| size <= MAX_FCS_PREALLOC_SIZE)
        .and_then(|size| usize::try_from(size).ok())
        .map(Vec::with_capacity)
        .unwrap_or_default();

    while !decoder.is_finished() {
        decoder.decode_blocks(&mut source, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;

        decoder.collect_into(&mut output);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_without_fcs_matches_original() {
        let compressed = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("frame initialization failed");

        // The legacy streaming-generated dictionary does not declare an FCS.
        assert_eq!(decoder.content_size(), None);

        let decoded = decompress(compressed).expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[cfg(feature = "dictionary-build")]
    #[test]
    fn decompress_with_fcs_matches_original() {
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        // One-shot compression declares the uncompressed frame content size.
        let compressed = zstd::bulk::compress(expected, 3).expect("zstd compression failed");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("frame initialization failed");

        assert_eq!(decoder.content_size(), Some(expected.len() as u64));

        let decoded = decompress(&compressed).expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }

    #[cfg(feature = "dictionary-build")]
    #[test]
    #[ignore]
    fn compare_embedded_and_one_shot_zstd_metadata() {
        let current = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.zstd");
        let cbor = include_bytes!("../dictionary_lib/dicts/dictionary_maxlength.cbor");

        let generated = zstd::bulk::compress(cbor, 19).expect("zstd compression failed");

        let mut current_decoder = FrameDecoder::new();
        current_decoder
            .init(current.as_slice())
            .expect("current frame initialization failed");

        let mut generated_decoder = FrameDecoder::new();
        generated_decoder
            .init(generated.as_slice())
            .expect("generated frame initialization failed");

        println!("CBOR size:                {} bytes", cbor.len());
        println!();
        println!("Current .zstd:");
        println!("  compressed size:        {} bytes", current.len());
        println!(
            "  frame content size:     {:?}",
            current_decoder.content_size()
        );
        println!();
        println!("One-shot .zstd:");
        println!("  compressed size:        {} bytes", generated.len());
        println!(
            "  frame content size:     {:?}",
            generated_decoder.content_size()
        );

        assert_eq!(current_decoder.content_size(), None);
        assert_eq!(generated_decoder.content_size(), Some(cbor.len() as u64));
    }

    #[cfg(feature = "dictionary-build")]
    #[test]
    #[ignore]
    fn inspect_csharp_zstd_metadata() {
        use std::fs;

        let json_path = r"R:\Media\dictionary_maxlength.json";
        let zstd_path = r"R:\Media\dictionary_maxlength.zstd";

        let json = fs::read(json_path).expect("failed to read C# JSON artifact");
        let compressed = fs::read(zstd_path).expect("failed to read C# Zstd artifact");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("C# Zstd frame initialization failed");

        let content_size = decoder.content_size();

        let decoded = decompress(&compressed).expect("C# Zstd decompression failed");

        println!("C# debug JSON size:       {} bytes", json.len());
        println!("C# .zstd size:            {} bytes", compressed.len());
        println!("C# frame content size:    {content_size:?}");
        println!("Rust decoded size:        {} bytes", decoded.len());

        assert_eq!(
            content_size,
            Some(decoded.len() as u64),
            "C# Zstd FCS should match the actual decompressed payload size"
        );
    }
}

#[cfg(test)]
mod regression_tests;
