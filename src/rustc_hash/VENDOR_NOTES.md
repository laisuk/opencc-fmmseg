# Vendoring notes: rustc-hash

## Upstream

- Project: `rustc-hash`
- Version: `2.1.3`
- Repository: <https://github.com/rust-lang/rustc-hash>
- Original license: MIT OR Apache-2.0
- Original upstream MSRV: Rust 1.77, verified from `rust-version = "1.77"`
  in both `Cargo.toml` and `Cargo.toml.orig` in the official 2.1.3 source
  distribution.
- Local compatibility target: Rust 1.75.0.

The comparison baseline is the official published source archive:
<https://static.crates.io/crates/rustc-hash/rustc-hash-2.1.3.crate>.

## Vendoring approach

The code is integrated as a private internal Rust module, declared by
`mod rustc_hash;` in `src/lib.rs`, rather than as a separate Cargo crate.
No separate `rustc-hash` Cargo dependency is required. Internal consumers use
`crate::rustc_hash`; existing public OpenCC APIs remain unchanged.

The upstream hashing algorithm and expected hash outputs must remain unchanged.
Preserve the upstream behavior for the corresponding architecture and stable
configuration; this does not imply identical outputs across architectures or
support for upstream optional nightly behavior.

## Local modifications relative to 2.1.3

- Upstream `src/lib.rs` becomes `src/rustc_hash/mod.rs`, with a local provenance,
  license, and compatibility header. The standalone crate examples in the module,
  `FxBuildHasher`, and `FxSeededState` documentation are removed.
- Crate-root imports in the vendored modules and their tests use `super::`
  instead of `crate::`. The test `HashBytes` implementation uses the already
  imported `Hasher` name instead of `core::hash::Hasher`. Consumer imports use
  `crate::rustc_hash`.
- The bulk byte loop replaces `split_first_chunk::<16>()` with a
  `while bulk.len() >= 16` guard and `bulk.split_at(16)`, compatible with Rust
  1.75.0. The same 16-byte chunks, ordering, mixing, remainder, and suffix are
  retained. Redundant references/parentheses in chunk conversions are removed,
  and `suffix[0..8]` becomes `suffix[..8]`.
- Standalone `#![no_std]`, nightly feature attributes (`const_default`,
  `const_trait_impl`, `derive_const`, `hasher_prefixfree_extras`), and
  `#![allow(rustc::default_hash_types)]` are removed. This module uses `std`
  unconditionally; `std` feature guards on imports, aliases, and seeded exports
  are removed, and seeded exports are combined into one re-export.
- Optional `rand` integration (`extern crate rand`, `random_state`, and its
  re-exports) is omitted. Nightly-only `Hasher::write_length_prefix` and
  `Hasher::write_str` overrides are omitted. `FxHasher` and `FxBuildHasher`
  derive ordinary `Default` unconditionally instead of selecting between
  ordinary and nightly const derives.
- Warning suppressions already present locally are `#[allow(dead_code)]` on
  `mod seeded_state`, `FxHashSet`, the seeded map/set aliases, and
  `FxSeededState::with_seed`; the combined seeded re-export has
  `#[allow(unused_imports)]`.
- Other differences are whitespace and minor comment wording edits (the MCG
  article and the bit-reversal/rotation commentary). Hash constants, mixing
  operations, and upstream hash regression expectations are retained.

## Seeded hashing

`seeded_state.rs` and its functionality are preserved, including
`FxSeededState::with_seed`, seeded map/set aliases, `BuildHasher`, and its three
unit tests. The source adaptations are the paths, documentation, feature guards,
and warning suppressions listed above. Unused APIs in a private internal module
may produce dead-code warnings; the local suppressions retain those APIs without
requiring all consumers to use them. Do not trim, rewrite, or remove seeded
hashing merely to silence warnings.

## Compatibility verification

The user successfully ran `cargo test` with `rustc 1.75.0` on Windows 7 x86 (32-bit). User-reported results across unit
tests, integration tests, and
documentation tests were:

- 253 tests passed.
- 0 failed.
- 20 ignored.
- The 32-bit hash regression tests passed.

This records the user's completed verification, not a rerun performed while
writing these notes. Ignored tests were not verified by that run. Other operating
systems, architectures, compiler versions, and optional upstream configurations
are not established as tested by these results. Package listing or package
verification on another host does not establish Windows 7 x86 compatibility.

## Licenses and packaging

`LICENSE-MIT` and `LICENSE-APACHE` were already present and verified byte for byte
against the official 2.1.3 source distribution. They retain their original text.
Their SHA-256 digests are:

- `LICENSE-MIT`: `30fefc3a7d6a0041541858293bcbea2dde4caa4c0a5802f996a7f7e8c0085652`
- `LICENSE-APACHE`: `95bd3988beee069fa2848f648dab43cc6e0b2add2ad6bcb17360caf749802bcc`

The root manifest explicitly includes this file and both upstream licenses in
the published package. Verify with `cargo package --list --allow-dirty` and,
when the environment permits, `cargo package --allow-dirty`. Do not publish as
part of this verification.

## Maintenance

For an upstream update, obtain the exact release source distribution and compare
it against both the current upstream baseline and these local files. Review
changes individually and selectively backport fixes while retaining Rust 1.75.0
compatibility, the private module paths, seeded hashing, and original licenses.
Update provenance and this modification inventory to reflect the actual source.
Do not automatically adopt upstream MSRV increases, optional dependencies,
nightly features, or algorithm/output changes.

Preserve hash-output stability for each supported architecture and retain the
regression expectations. Rerun the full suite on Windows 7 x86 with Rust 1.75.0,
including the 32-bit hash regression tests, after source backports; verify other
targets separately and label their results explicitly. Recheck license text
against the selected release and confirm all three documentation/license files
remain in the Cargo package listing.
