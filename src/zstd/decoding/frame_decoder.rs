//! Framedecoder is the main low-level struct users interact with to decode zstd frames
//!
//! Zstandard compressed data is made of one or more frames. Each frame is independent and can be
//! decompressed independently of other frames. This module contains structures
//! and utilities that can be used to decode a frame.

use super::frame;
use crate::zstd::decoding;
use crate::zstd::decoding::errors::FrameDecoderError;
use crate::zstd::decoding::scratch::DecoderScratch;
use std::io::{Error, Read};
use std::vec::Vec;

/// The default maximum window size, in bytes, that a [FrameDecoder] accepts.
///
/// Defaults to 100mb to bound allocation for malformed or hostile frames. The
/// spec permits far larger windows; this internal decoder keeps a fixed limit.
pub const DEFAULT_MAX_WINDOW_SIZE: u64 = 1024 * 1024 * 100;

/// Low level Zstandard decoder that can be used to decompress frames with fine control over when and how many bytes are decoded.
///
/// This decoder is able to decode frames only partially and gives control
/// over how many bytes/blocks will be decoded at a time (so you don't have to decode a 10GB file into memory all at once).
/// It reads bytes as needed from a provided source and can be read from to collect partial results.
pub struct FrameDecoder {
    state: Option<FrameDecoderState>,
}

struct FrameDecoderState {
    pub frame_header: frame::FrameHeader,
    decoder_scratch: DecoderScratch,
    frame_finished: bool,
    check_sum: Option<u32>,
}

pub enum BlockDecodingStrategy {
    UptoBytes(usize),
}

impl FrameDecoderState {
    fn new(
        source: impl Read,
        max_window_size: u64,
    ) -> Result<FrameDecoderState, FrameDecoderError> {
        let (frame, _) = frame::read_frame_header(source)?;
        let window_size = frame.window_size()?;
        Self::check_window_size(window_size, max_window_size)?;
        Ok(FrameDecoderState {
            frame_header: frame,
            frame_finished: false,
            decoder_scratch: DecoderScratch::new(window_size as usize),
            check_sum: None,
        })
    }

    fn reset(&mut self, source: impl Read, max_window_size: u64) -> Result<(), FrameDecoderError> {
        let (frame_header, _) = frame::read_frame_header(source)?;
        let window_size = frame_header.window_size()?;
        Self::check_window_size(window_size, max_window_size)?;

        self.frame_header = frame_header;
        self.frame_finished = false;
        self.decoder_scratch.reset(window_size as usize);
        self.check_sum = None;
        Ok(())
    }

    /// Reject a frame whose declared window exceeds the limit, before allocation.
    fn check_window_size(window_size: u64, max_window_size: u64) -> Result<(), FrameDecoderError> {
        if window_size > max_window_size {
            return Err(FrameDecoderError::WindowSizeTooBig {
                requested: window_size,
                max: max_window_size,
            });
        }
        Ok(())
    }
}

impl FrameDecoder {
    /// This will create a new decoder without allocating anything yet.
    /// init() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    pub fn new() -> FrameDecoder {
        FrameDecoder { state: None }
    }

    /// init() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    ///
    /// Note that all bytes currently in the decodebuffer from any previous frame will be lost. Collect them with collect()
    pub fn init(&mut self, source: impl Read) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        let state = match &mut self.state {
            Some(s) => {
                s.reset(source, DEFAULT_MAX_WINDOW_SIZE)?;
                s
            }
            None => {
                self.state = Some(FrameDecoderState::new(source, DEFAULT_MAX_WINDOW_SIZE)?);
                self.state.as_mut().unwrap()
            }
        };
        if let Some(dict_id) = state.frame_header.dictionary_id() {
            // The entry points never provide a Zstandard decoding dictionary.
            return Err(err::DictNotProvided { dict_id });
        }
        Ok(())
    }

    /// Whether the current frames last block has been decoded yet
    /// If this returns true you can call the drain* functions to get all content
    /// (the read() function will drain automatically if this returns true)
    pub fn is_finished(&self) -> bool {
        let state = match &self.state {
            None => return true,
            Some(s) => s,
        };
        if state.frame_header.descriptor.content_checksum_flag() {
            state.frame_finished && state.check_sum.is_some()
        } else {
            state.frame_finished
        }
    }

    /// Decodes blocks from a reader. It requires that the framedecoder has been initialized first.
    /// The Strategy influences how many blocks will be decoded before the function returns
    /// The byte target bounds how much output is decoded per call.
    pub fn decode_blocks(
        &mut self,
        mut source: impl Read,
        strat: BlockDecodingStrategy,
    ) -> Result<bool, FrameDecoderError> {
        use FrameDecoderError as err;
        let state = self.state.as_mut().ok_or(err::NotYetInitialized)?;

        let mut block_dec = decoding::block_decoder::new();

        let buffer_size_before = state.decoder_scratch.buffer.len();
        loop {
            let (block_header, _) = block_dec
                .read_block_header(&mut source)
                .map_err(err::FailedToReadBlockHeader)?;


            block_dec
                .decode_block_content(&block_header, &mut state.decoder_scratch, &mut source)
                .map_err(err::FailedToReadBlockBody)?;


            if block_header.last_block {
                state.frame_finished = true;
                if state.frame_header.descriptor.content_checksum_flag() {
                    let mut chksum = [0u8; 4];
                    source
                        .read_exact(&mut chksum)
                        .map_err(err::FailedToReadChecksum)?;
                    let chksum = u32::from_le_bytes(chksum);
                    state.check_sum = Some(chksum);
                }
                break;
            }

            match strat {
                BlockDecodingStrategy::UptoBytes(n) => {
                    if state.decoder_scratch.buffer.len() - buffer_size_before >= n {
                        break;
                    }
                }
            }
        }

        Ok(state.frame_finished)
    }

    /// Collect bytes and retain window_size bytes while decoding is still going on.
    /// After decoding of the frame (is_finished() == true) has finished it will collect all remaining bytes
    pub fn collect(&mut self) -> Option<Vec<u8>> {
        let finished = self.is_finished();
        let state = self.state.as_mut()?;
        if finished {
            Some(state.decoder_scratch.buffer.drain())
        } else {
            state.decoder_scratch.buffer.drain_to_window_size()
        }
    }

    /// How many bytes can currently be collected from the decodebuffer, while decoding is going on this will be lower than the actual decodbuffer size
    /// because window_size bytes need to be retained for decoding.
    /// After decoding of the frame (is_finished() == true) has finished it will report all remaining bytes
    pub fn can_collect(&self) -> usize {
        let finished = self.is_finished();
        let state = match &self.state {
            None => return 0,
            Some(s) => s,
        };
        if finished {
            state.decoder_scratch.buffer.can_drain()
        } else {
            state
                .decoder_scratch
                .buffer
                .can_drain_to_window_size()
                .unwrap_or(0)
        }
    }

    /// Decode multiple frames into the output slice.
    ///
    /// `input` must contain an exact number of frames.
    ///
    /// `output` must be large enough to hold the decompressed data. If you don't know
    /// how large the output will be, use [`FrameDecoder::decode_blocks`] instead.
    ///
    /// This calls [`FrameDecoder::init`], and all bytes currently in the decoder will be lost.
    ///
    /// Returns the number of bytes written to `output`.
    pub fn decode_all(
        &mut self,
        mut input: &[u8],
        mut output: &mut [u8],
    ) -> Result<usize, FrameDecoderError> {
        let mut total_bytes_written = 0;
        while !input.is_empty() {
            match self.init(&mut input) {
                Ok(_) => {}
                Err(FrameDecoderError::ReadFrameHeaderError(
                    crate::zstd::decoding::errors::ReadFrameHeaderError::SkipFrame { length, .. },
                )) => {
                    input = input
                        .get(length as usize..)
                        .ok_or(FrameDecoderError::FailedToSkipFrame)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            loop {
                self.decode_blocks(&mut input, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;
                let bytes_written = self
                    .read(output)
                    .map_err(FrameDecoderError::FailedToDrainDecodebuffer)?;
                output = &mut output[bytes_written..];
                total_bytes_written += bytes_written;
                if self.can_collect() != 0 {
                    return Err(FrameDecoderError::TargetTooSmall);
                }
                if self.is_finished() {
                    break;
                }
            }
        }

        Ok(total_bytes_written)
    }
}

/// Read bytes from the decode_buffer that are no longer needed. While the frame is not yet finished
/// this will retain window_size bytes, else it will drain it completely
impl Read for FrameDecoder {
    fn read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let state = match &mut self.state {
            None => return Ok(0),
            Some(s) => s,
        };
        if state.frame_finished {
            state.decoder_scratch.buffer.read_all(target)
        } else {
            state.decoder_scratch.buffer.read(target)
        }
    }
}
