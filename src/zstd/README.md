# Internal Zstandard decoder

Adapted from ruzstd 0.9.0 for opencc-fmmseg; requires Rust 1.75.
See LICENSE-RUZSTD and NOTICE.md for attribution.

## Entry points and retained dependencies

- `decompress()`: initializes a frame, calls `decode_blocks(UptoBytes(...))`,
  and collects output. It does not depend on declared content size.
- `decompress_into()`: calls `decode_all()`, which also needs slice reads,
  `can_collect()`, frame reset, and skippable-frame handling.
- `decompress_exact()`: allocates the requested capacity and delegates to
  `decompress_into()`. It truncates to the bytes written; an undersized output
  returns an error.

Both paths require frame/block parsing, literal and sequence decoding, FSE and
Huffman tables, bit readers, scratch state, and the ring buffer. Collection
retains the history window until the frame finishes. Table resets remain
necessary between frames; repeated tables within a frame remain supported.

## Preserved behavior

The generic path decodes one frame and ignores trailing input. The sized paths
decode concatenated frames and skip skippable frames. A leading skippable frame
is an error for the generic path. These are intentionally different existing
behaviors.

Checksum bytes are consumed, and truncated checksums are errors. Checksums are
not validated; the upstream hash feature was never enabled by this crate.
The decoder keeps its 100 MiB window limit and format-level window validation.

OpenCC's CBOR dictionaries are payloads, not Zstandard decoding dictionaries.
The entry points never provided decoding dictionaries, so nonzero dictionary
IDs still return `DictNotProvided`. Invalid offsets retain their previous
errors, including `NotEnoughBytesInDictionary` with zero available bytes.

## Trimming scope

Removed the dictionary parser/registration/table-copy APIs, general-purpose
frame getters and alternate output APIs, writer support and writer-only tests,
inactive checksum hashing hooks, unused ring-buffer alternatives, unreachable
error variants, and the std I/O compatibility module. The active decoding
algorithms are retained.

Regression tests cover embedded data equality, unknown content size, history
across collection boundaries, frame/skip boundaries, output capacity, checksum
consumption, truncation, dictionary IDs, and window limits. Ring-buffer tests
retain a test-only checked wrapper around the production copy routine.

The sized entry points and their dependencies can still produce dead-code
warnings in non-test builds. They are deliberately retained and tested.
