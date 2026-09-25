use super::decoding::errors::ReadFrameHeaderError;
use super::{decompress, FrameDecoderError};

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

    let large_window = [0x28, 0xb5, 0x2f, 0xfd, 0, 17 << 3];
    assert!(matches!(
        decompress(&large_window),
        Err(FrameDecoderError::WindowSizeTooBig { .. })
    ));
}

#[test]
fn unknown_size_across_collection_and_history_boundaries() {
    use std::io::Write;

    let input: Vec<u8> = (0..3 * 1024 * 1024)
        .map(|i| ((i * 31 + i / 97) % 251) as u8)
        .collect();

    let mut encoder = zstd::Encoder::new(Vec::new(), 3).unwrap();
    encoder.window_log(17).unwrap();
    encoder.include_checksum(true).unwrap();
    encoder.write_all(&input).unwrap();
    let compressed = encoder.finish().unwrap();

    let (header, _) =
        super::decoding::frame::read_frame_header(compressed.as_slice()).unwrap();
    assert_eq!(header.frame_content_size(), 0);
    assert_eq!(decompress(&compressed).unwrap(), input);
}

#[test]
fn invalid_history_offsets_preserve_errors() {
    use super::decoding::{decode_buffer::DecodeBuffer, errors::DecodeBufferError};
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
