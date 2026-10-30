## opencc-rs
### OpenCC Rust
A command line Open Chinese converter

### Compatibility processing

Use `--norm-compat` for CJK Compatibility Ideographs or
`--norm-compat-extended` for the combined curated compatibility normalization.
Use `--detofu [LEVEL]` for display fallback after conversion, with an optional
`--detofu-file FILE` for custom overrides.

The pipeline always runs optional normalization, OpenCC conversion with optional
punctuation handling, then optional DeTofu. Enabled normalization borrows unchanged
input; unchanged DeTofu output reuses the converted String. These optimizations
require no new flags and do not change mappings, defaults, or pipeline order.
Changed text still needs owned output, and speed varies by workload.

See the [main usage documentation](../../README.md),
[DeTofu guide](../../DETOFU_USER_GUIDE.md), and
[allocation benchmark review](../../benches/allocation_review.md).
