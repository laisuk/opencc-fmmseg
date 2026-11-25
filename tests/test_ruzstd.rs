// Repository-only tests: the original CBOR fixture is not shipped in the crate.
// Compile the private decoder here without exposing it through the library API.
#[allow(clippy::upper_case_acronyms)]
#[path = "../src/zstd/mod.rs"]
mod zstd;

use crate::zstd::decoding::errors::ReadFrameHeaderError;
use crate::zstd::decoding::FrameDecoder;
use crate::zstd::{decompress, FrameDecoderError};

#[test]
fn decompress_embedded_matches_original_and_has_valid_fcs() {
    let compressed = include_bytes!("../src/dictionary_lib/dicts/dictionary_maxlength.zstd");
    let expected = include_bytes!("../src/dictionary_lib/dicts/dictionary_maxlength.cbor");

    let mut decoder = FrameDecoder::new();
    decoder
        .init(compressed.as_slice())
        .expect("frame initialization failed");

    let content_size = decoder.content_size();

    let decoded = decompress(compressed).expect("zstd decompression failed");

    // Verify the embedded artifact's actual payload first.
    assert_eq!(decoded.as_slice(), expected);

    // The embedded artifact is generated with one-shot Zstd compression,
    // so its FCS must describe the actual decompressed payload.
    assert_eq!(
        content_size,
        Some(decoded.len() as u64),
        "embedded Zstd FCS should match the actual decompressed size"
    );
}

#[cfg(feature = "dictionary-build")]
#[test]
fn decompress_with_fcs_matches_original() {
    let expected = include_bytes!("../src/dictionary_lib/dicts/dictionary_maxlength.cbor");

    // One-shot compression declares the uncompressed frame content size.
    let compressed = ::zstd::bulk::compress(expected, 3).expect("zstd compression failed");

    let mut decoder = FrameDecoder::new();
    decoder
        .init(compressed.as_slice())
        .expect("frame initialization failed");

    assert_eq!(decoder.content_size(), Some(expected.len() as u64));

    let decoded = decompress(&compressed).expect("zstd decompression failed");

    assert_eq!(decoded.as_slice(), expected);
}

fn raw_frame(data: &[u8], checksum: bool) -> Vec<u8> {
    assert!(data.len() < 256);
    let mut frame = vec![
        0x28,
        0xb5,
        0x2f,
        0xfd,
        0x20 | if checksum { 4 } else { 0 },
        data.len() as u8,
    ];
    let block = ((data.len() as u32) << 3) | 1;
    frame.extend_from_slice(&block.to_le_bytes()[..3]);
    frame.extend_from_slice(data);
    if checksum {
        // Existing behavior: consume checksum bytes without validating them.
        frame.extend_from_slice(&[0; 4]);
    }
    frame
}

#[test]
fn frame_boundaries() {
    let first = raw_frame(b"first", false);
    let mut joined = first.clone();
    joined.extend_from_slice(&raw_frame(b"second", true));

    // The canonical entry point decodes a single Zstandard frame.
    assert_eq!(decompress(&joined).unwrap(), b"first");
    assert!(decompress(&[]).is_err());
    assert_eq!(decompress(&raw_frame(b"", false)).unwrap(), b"");

    // Last RLE block, regenerated size 100, repeated byte 9.
    let rle = [0x28, 0xb5, 0x2f, 0xfd, 0x20, 100, 0x23, 0x03, 0, 9];
    assert_eq!(decompress(&rle).unwrap(), vec![9; 100]);
}

#[test]
fn skippable_frames_keep_existing_entry_point_behavior() {
    let mut skip = 0x184d2a50u32.to_le_bytes().to_vec();
    skip.extend_from_slice(&3u32.to_le_bytes());
    skip.extend_from_slice(b"xyz");
    let mut input = skip.clone();
    input.extend_from_slice(&raw_frame(b"hello", false));
    input.extend_from_slice(&skip);

    assert!(matches!(
        decompress(&input),
        Err(FrameDecoderError::ReadFrameHeaderError(
            ReadFrameHeaderError::SkipFrame { .. }
        ))
    ));
}

#[test]
fn checksum_consumption_and_truncation() {
    let frame = raw_frame(b"checksum", true);
    assert_eq!(decompress(&frame).unwrap(), b"checksum");

    for end in 0..frame.len() {
        assert!(decompress(&frame[..end]).is_err());
    }
}

#[test]
fn dictionary_ids_and_window_limit() {
    let with_dictionary = [0x28, 0xb5, 0x2f, 0xfd, 0x21, 7, 0];
    assert!(matches!(
        decompress(&with_dictionary),
        Err(FrameDecoderError::DictNotProvided { dict_id: 7 })
    ));

    let mut max_window = window_frame(17 << 3);
    append_block(&mut max_window, 0, 2, true, b"ok");
    assert_eq!(decompress(&max_window).unwrap(), b"ok");

    let large_window = [0x28, 0xb5, 0x2f, 0xfd, 0, (17 << 3) | 1];
    assert!(matches!(
        decompress(&large_window),
        Err(FrameDecoderError::WindowSizeTooBig { .. })
    ));
}

#[cfg(feature = "dictionary-build")]
#[test]
fn unknown_size_across_collection_and_history_boundaries() {
    use std::io::Write;

    let input: Vec<u8> = (0..3 * 1024 * 1024)
        .map(|i| ((i * 31 + i / 97) % 251) as u8)
        .collect();

    let mut encoder = ::zstd::Encoder::new(Vec::new(), 3).unwrap();
    encoder.window_log(17).unwrap();
    encoder.include_checksum(true).unwrap();
    encoder.write_all(&input).unwrap();
    let compressed = encoder.finish().unwrap();

    let (header, _) = zstd::decoding::frame::read_frame_header(compressed.as_slice()).unwrap();

    assert_eq!(header.frame_content_size(), 0);
    assert_eq!(decompress(&compressed).unwrap(), input);
}

#[test]
fn invalid_history_offsets_preserve_errors() {
    use crate::zstd::decoding::{decode_buffer::DecodeBuffer, errors::DecodeBufferError};
    let mut buffer = DecodeBuffer::new(4);
    buffer.push(b"ab");
    assert!(matches!(
        buffer.repeat(3, 1),
        Err(DecodeBufferError::NotEnoughBytesInDictionary { got: 0, need: 1 })
    ));
    buffer.push(b"cde");
    assert!(matches!(
        buffer.repeat(6, 1),
        Err(DecodeBufferError::OffsetTooBig {
            offset: 6,
            buf_len: 5
        })
    ));
}

#[test]
fn collection_appends_wrapped_output_and_preserves_history() {
    use crate::zstd::decoding::decode_buffer::DecodeBuffer;

    let mut buffer = DecodeBuffer::new(8);
    let initial: Vec<u8> = (0..24).collect();
    let mut output = b"prefix".to_vec();
    buffer.push(&initial);
    buffer.drain_into(&mut output, 16);
    assert_eq!(buffer.len(), 8);

    // Repeating the retained history wraps the ring's occupied region.
    buffer.repeat(8, 24).unwrap();
    buffer.drain_into(&mut output, 24);
    assert_eq!(buffer.len(), 8);
    buffer.repeat(8, 8).unwrap();
    buffer.drain_into(&mut output, 16);
    assert_eq!(buffer.len(), 0);

    let mut expected = b"prefix".to_vec();
    expected.extend_from_slice(&initial);
    expected.extend_from_slice(&initial[16..].repeat(4));
    assert_eq!(output, expected);

    buffer.drain_into(&mut output, 0);
    assert_eq!(output, expected);
}

#[test]
fn fcs_remains_a_capacity_hint() {
    for declared in [0, 2, 30] {
        let mut frame = raw_frame(b"actual output", true);
        frame[5] = declared;
        assert_eq!(decompress(&frame).unwrap(), b"actual output");
    }
}

fn window_frame(descriptor: u8) -> Vec<u8> {
    vec![0x28, 0xb5, 0x2f, 0xfd, 0, descriptor]
}

fn append_block(frame: &mut Vec<u8>, kind: u32, size: u32, last: bool, body: &[u8]) {
    let header = (size << 3) | (kind << 1) | u32::from(last);
    frame.extend_from_slice(&header.to_le_bytes()[..3]);
    frame.extend_from_slice(body);
}

#[test]
fn block_bodies_are_bounded_by_explicit_window_before_reading() {
    use zstd::decoding::errors::{DecodeBlockContentError, DecompressBlockError};
    for kind in 0..=2 {
        let mut frame = window_frame(0);
        append_block(&mut frame, kind, 1025, true, b"");
        assert!(matches!(
            decompress(&frame),
            Err(FrameDecoderError::FailedToReadBlockBody(
                DecodeBlockContentError::DecompressBlockError(
                    DecompressBlockError::BlockSizeLimitExceeded {
                        size: 1025,
                        max: 1024
                    }
                )
            ))
        ));
    }
}

// RLE literals fixture encoding adapted from ruzstd commit 91a3a6eb (MIT).
fn compressed_rle_literals(size: u32, sequences: &[u8]) -> Vec<u8> {
    let header = (size << 4) | 0x0d;
    let mut body = header.to_le_bytes()[..3].to_vec();
    body.push(b'x');
    body.extend_from_slice(sequences);
    body
}

#[test]
fn compressed_literals_without_sequences_obey_block_limit() {
    use zstd::decoding::errors::{DecodeBlockContentError, DecompressBlockError};
    let decode = |size| {
        let mut frame = window_frame(0);
        let body = compressed_rle_literals(size, &[0]);
        append_block(&mut frame, 2, body.len() as u32, true, &body);
        decompress(&frame)
    };
    assert_eq!(decode(1024).unwrap(), vec![b'x'; 1024]);
    assert!(matches!(
        decode(1025),
        Err(FrameDecoderError::FailedToReadBlockBody(
            DecodeBlockContentError::DecompressBlockError(
                DecompressBlockError::BlockSizeLimitExceeded {
                    size: 1025,
                    max: 1024
                }
            )
        ))
    ));
}

#[test]
fn trailing_literals_count_toward_block_limit() {
    use zstd::decoding::errors::{
        DecodeBlockContentError, DecompressBlockError, ExecuteSequencesError,
    };
    let decode = |size| {
        let mut frame = window_frame(0);
        append_block(&mut frame, 0, 4, false, b"abcd");
        // Adapted from ruzstd commit 91a3a6eb (MIT). One match of length
        // three at offset four, leaving all literals to be appended last.
        let body = compressed_rle_literals(size, &[1, 0x54, 0, 0, 0, 1]);
        append_block(&mut frame, 2, body.len() as u32, true, &body);
        decompress(&frame)
    };
    let mut expected = b"abcdabc".to_vec();
    expected.extend_from_slice(&[b'x'; 1021]);
    assert_eq!(decode(1021).unwrap(), expected);
    assert!(matches!(
        decode(1022),
        Err(FrameDecoderError::FailedToReadBlockBody(
            DecodeBlockContentError::DecompressBlockError(
                DecompressBlockError::ExecuteSequencesError(
                    ExecuteSequencesError::BlockSizeLimitExceeded { max: 1024 }
                )
            )
        ))
    ));
}
