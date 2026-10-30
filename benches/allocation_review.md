# Allocation review

Baseline: `87afddd02ecd70c2ebdb4699de4d1ebb87b1182c`.

## Initial audit (before implementation)

| Candidate | Expected benefit | Risk / decision |
| --- | --- | --- |
| Compatibility normalization | Borrow unchanged input; copy the UTF-8 prefix once on first replacement | Low: use byte indices from `char_indices`; preserve existing per-character precedence and no chaining |
| Extended normalization | Same lazy allocation with existing lookup methods | Low: preserve extended-only versus combined modes |
| DeTofu owned-return convenience methods | Scan for first actual change, bulk-copy unchanged prefix | Benchmark first: custom overrides must precede BMP guard; identity overrides must suppress built-ins |
| DeTofu `*_into` | Already reuses allocation and appends | Keep existing loops; avoid additional scans or changing reserve behavior |
| CLI normalization | Avoid intermediate owned output when enabled modes change nothing | Needs additive public Cow methods because CLI is a separate crate; retain all existing signatures |
| CLI final DeTofu | Move converted output through unchanged when no fallback applies | Use borrowed result to retain the original converted String |
| FMM/union lookup | Already borrows character slices and dictionary values, writes directly into output | Keep algorithm, starter gates, dictionary precedence, and IDS boundaries intact |
| Segmentation/multiple rounds | Character/range vectors and per-round Strings are possible future targets | Higher risk; rounds change matching input, parallel chunks require separate outputs; no speculative rewrite |
| Punctuation | Integrated dictionaries on main path; legacy regex helper is unused | No benefit from optimizing the unused helper |

The benchmark example includes the real CLI pipeline source and a counting system
allocator. Allocation calls include reallocations; bytes are total requested bytes,
not peak live memory. Timing disables atomic counter updates. Inputs and reusable
maps/buffers are prepared outside measurements. Warm results are medians of seven
samples; cold mode is run in seven fresh processes. Serial conversion provides
deterministic allocation attribution; default parallel conversion is also checked.

The actual FMM path uses starter-union masks/caps plus hash lookups keyed by
borrowed `&[char]` slices, rather than constructing trie keys or temporary Strings.
Phrase hits use `push_str` on borrowed values; unmatched scalars use `push`.
Short inputs fall back to serial processing when there is at most one range chunk,
but still use the existing character/range buffers. With IDS preservation enabled,
complete IDS ranges bypass matching. Custom dictionary overlays insert pairs with
last-wins semantics and rebuild metadata; changing or skipping conversion solely on an ASCII check would
be incorrect for arbitrary custom dictionaries. Character-only `st`/`ts` helpers
also collect a character vector, but their script-detection caller is bounded to
a small input prefix; this review does not rewrite that separate path.

## Implementation

- `src/lazy_replace.rs::{replace_chars, first_replacement}` and
  `Replacement::start_output`: one scalar traversal. Find the first
  actual change, derive its byte offset from `Chars::as_str`, copy the unchanged
  prefix, and map the remaining iterator. No complete pre-scan, unsafe code, or
  remapping of replacement characters. Capacity accounts for the first mapping's
  UTF-8 expansion; subsequent expansion uses normal String growth.
- `CompatIdeographs::{normalize, normalize_cow}` and
  `UnicodeCompat::{normalize, normalize_all, normalize_cow, normalize_all_cow}`:
  reuse existing character lookups. Owned convenience methods call `into_owned`.
- `DetofuMap::detofu_cow` and internal `detofu::{detofu, detofu_cow,
  builtin_fallback}`: preserve custom overrides, identity overrides, thresholds,
  and the built-in BMP guard. On a change, DeTofu uses its established append
  writer for the remaining suffix. The existing append/reuse loops are untouched.
  `DetofuMap::detofu` is entirely unchanged: a measured
  regression on frequent width-expanding custom matches outweighed the benefit
  of making this owned convenience method lazy. The borrowed API remains useful
  for custom-map pipelines and no-match inputs.
- `OpenCC`: additive `normalize_compat_cow`, `normalize_unicode_compat_cow`,
  `normalize_compat_extended_cow`, and `detofu_cow` methods. Lifetimes depend only
  on the input. Existing Rust signatures, C exports, versions, and dependencies
  are unchanged. Public borrowed methods are needed across the CLI crate boundary.
- `text_converter::apply_conversion_pipeline`: use borrowed normalization;
  move the converted String out when DeTofu returns a borrowed result. Order is
  still normalization, conversion/punctuation, then DeTofu.

## Environment and reproduction

Windows x86_64 MSVC, Intel Core i5-13400 (10 cores / 16 logical processors),
rustc 1.99.1 stable, LLVM 23.1.1. Both builds use the committed release profile:
opt-level 3, one codegen unit, fat LTO, panic abort, no incremental compilation.
No RUSTFLAGS changes or dependency changes.

```powershell
cargo +stable build --release --example allocation_review --locked
& target/release/examples/allocation_review.exe benches/ChangFeng_Simp.txt
1..7 | ForEach-Object { & target/release/examples/allocation_review.exe --cold }
cargo +stable run --release --example allocation_scanners --locked
```

The optional novel argument is a user-provided, untracked UTF-8 corpus (1,130,220
scalars, 3,288,254 bytes here); it is not added to this change. Without that
argument all synthetic inputs, including exactly 3,000,000 scalars, are generated
by the example. To reproduce the baseline, extract the specified commit into an
isolated directory and copy `examples/allocation_review.rs` there. The same
harness compiles against both versions and includes each checkout's actual CLI
pipeline source. The baseline used here was also rebuilt in an isolated archive
under `target/allocation-baseline`.

`ALLOCATION_CASE` optionally restricts measurements to labels containing that
substring. Follow-up runs alternate baseline/optimized order, with three fresh
processes per version for ASCII, short text, mixed text, and novel cases. Each
process reports seven timing samples; the follow-up comparison uses the median of those three
medians. Builds and tests do not run concurrently with the recorded measurements.
Allocation counts are separately sampled seven times after warming each operation.

The counter adds an atomic flag read on allocation even when counting is disabled;
timings therefore describe the instrumented binaries, not an uninstrumented CLI.
Cold mode measures constructor and first use of each optional table separately;
it does not include a first conversion's lazy starter-union construction.
Extended-table cold time is incremental: the compatibility table is initialized
by the preceding basic-normalization call. No CPU affinity or power-state control
was applied; small timing differences should be treated as measurement variation.

## Warm benchmark results

Pipeline rows use s2t with punctuation and DeTofu; extended normalization is
enabled except where the row explicitly names a standalone operation. These are
serial pipeline measurements; parallel core checks follow below. Negative time
deltas mean faster. Repeated cases use three process medians of seven samples;
other rows use seven samples from the full input matrix.

| Case | Before | After | Time delta | Allocations | Requested bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| compat/ascii | 2.189 us | 0.079 us | -96.4% | 1 -> 1 | 3200 -> 3200 |
| pipeline_extended/ascii | 17.755 us | 14.012 us | -21.1% | 15 -> 13 | 64782 -> 58382 |
| pipeline_extended/simplified | 83.946 us | 84.358 us | 0.5% | 12 -> 10 | 49720 -> 35896 |
| pipeline_extended/traditional | 52.588 us | 54.560 us | 3.8% | 12 -> 10 | 49720 -> 35896 |
| pipeline_extended/rare_compat | 26.923 us | 26.262 us | -2.5% | 12 -> 11 | 31270 -> 27424 |
| pipeline_extended/mixed | 34.115 us | 29.624 us | -13.2% | 15 -> 13 | 86576 -> 75824 |
| pipeline_extended/supplementary | 22.573 us | 23.549 us | 4.3% | 11 -> 9 | 24822 -> 17910 |
| pipeline_extended/occasional_tofu | 26.932 us | 25.177 us | -6.5% | 12 -> 10 | 31263 -> 23576 |
| pipeline_custom/mixed | 36.274 us | 34.262 us | -5.5% | 16 -> 15 | 97328 -> 91961 |
| pipeline_extended/short | 0.544 us | 0.455 us | -16.4% | 6 -> 4 | 301 -> 223 |
| pipeline_extended/novel_3m | 120.291 ms | 112.496 ms | -6.5% | 23 -> 21 | 69018126 -> 51649702 |
| pipeline_extended/real_novel | 72.557 ms | 70.761 ms | -2.5% | 22 -> 21 | 28169452 -> 24881198 |

The standalone basic-normalization ASCII path uses a byte-level ASCII check.
Unlike arbitrary custom DeTofu mappings, this table cannot contain ASCII sources.

There is no universal speedup: standalone extended normalization on the real
novel measured 4.917 -> 5.383 ms (+9.5%), while its complete extended pipeline
improved by 2.5% and removed one 3.29 MB intermediate allocation. The unchanged
owned custom-map implementation measured 6.309 -> 7.362 us (+16.7%) on mixed
width-expanding input; the corresponding custom pipeline improved by 5.5% and
removed one allocation. These remaining timing regressions are recorded rather
than attributed to an unverified cause. Keeping the original owned writer avoids
the larger regression of the rejected lazy variants. The retained optimization
is justified by the measured end-to-end pipeline gains and deterministic memory
savings; standalone custom-map throughput is a limitation of this build comparison.

Core conversion sources and allocations are unchanged. Repeated serial core
conversion measured +1.1% for 3 million scalars and -1.7% for short text. Default
parallel core conversion measured -2.7% for 3 million scalars and +1.3% for short
text. These small differences are not claimed as FMM improvements.

## Scan strategy comparison

All strategies use the same curated combined character lookup and explicit
output pushes. The pre-scan strategy stops detection at the first change, then
traverses the entire input to build output. Complete-scan counts changes over all
input before a second traversal. The retained first-replacement strategy visits
each original scalar once and copies the already visited prefix as bytes.

| Input | First replacement | Early-exit pre-scan + output | Complete scan + output |
| --- | ---: | ---: | ---: |
| Unchanged CJK | 2.866 us | 2.695 us | 2.699 us |
| First character changes | 4.400 us | 4.407 us | 7.045 us |
| Middle changes | 7.982 us | 12.568 us | 15.517 us |
| Last character changes | 3.217 us | 7.596 us | 7.637 us |
| Dense changes | 0.826 us | 0.833 us | 1.284 us |
| 3-million-scalar unchanged input | 6.884 ms | 6.295 ms | 6.350 ms |

The single-visit approach has modest detection overhead on unchanged inputs
relative to the pre-scan alternatives, but avoids their repeated prefix mapping
when changes occur. It also avoids the eager baseline output construction.

## Cold initialization

Seven fresh processes per version; no warm timings are mixed into this table.

| Stage | Before median | After median |
| --- | ---: | ---: |
| OpenCC constructor | 24.083 ms | 22.378 ms |
| First compatibility-table call | 54.5 us | 56.8 us |
| First extended-table call, compatibility already initialized | 44.2 us | 40.8 us |
| First built-in DeTofu call | 125.6 us | 121.5 us |

Initialization is not optimized by this change; these startup differences should
not be interpreted as a cold-start optimization.

Raw results: [full before](allocation_before.csv), [full after](allocation_after.csv),
[full comparison](allocation_comparison.csv), [process repeats](allocation_repeats.csv),
[repeat comparison](allocation_repeat_comparison.csv), [scan comparison](allocation_scanners.csv),
[cold before](allocation_cold_before.txt), [cold after](allocation_cold_after.txt),
and [rejected custom-map trials](allocation_custom_trials.csv). The custom trials
record the extend suffix, explicit-push suffix, and append-suffix implementations
with a lazy owned custom-map wrapper; none of those owned wrappers is retained.

## Validation

`cargo test --workspace --all-features --locked` on Rust 1.75.0: **353 passed,
0 failed, 25 ignored** across unit, integration, and documentation tests. Ignored
tests are existing diagnostics, dictionary generators, and documentation examples.
The workspace suite also passed on stable before the final iterator refinement;
the final refinement and exhaustive scalar test are covered by the Rust 1.75 run.

Coverage includes all Unicode scalars against the existing normalization character
lookups, every built-in DeTofu entry at every level, custom/identity overrides,
empty/unchanged borrowed results, beginning/middle/end replacements, supplementary
characters, UTF-8 expansion/contraction, append and buffer reuse, input-only
lifetimes, existing String signatures, and CLI stage composition with punctuation,
IDS, regional conversion, and invalid configuration. Existing integration tests
cover custom dictionary last-wins behavior and the C API.

## Allocation behavior

For nonempty, unchanged input with tables already initialized:

| API/path | Before | After |
| --- | ---: | ---: |
| Existing String-returning normalization methods | 1 output allocation | 1 output allocation |
| Existing String-returning DeTofu methods | 1 output allocation | 1 output allocation |
| New borrowed normalization / DeTofu methods | Not available | 0 output allocations |
| DeTofu into a sufficiently large reusable String | 0 | 0 |
| Short CLI pipeline, extended normalization + DeTofu | 6 allocations / 301 bytes | 4 allocations / 223 bytes |
| 3-million-scalar CLI pipeline, same modes | 23 allocations / 69,018,126 bytes | 21 allocations / 51,649,702 bytes |

The large pipeline avoids 17,368,424 requested bytes (17.37 MB, 16.56 MiB).
The counter measures total requested bytes, including allocation growth; this is
not a claim about peak resident memory. Changed inputs still require owned output,
and variable-width custom replacements can require normal String growth.

## Deliberately not changed

- No Cow conversion of FMM output: deciding whether dictionary output changed
  adds comparisons in the tightest loop and complicates multiple rounds and IDS.
- No round fusion or vector/range rewrite: these buffers carry matching and
  delimiter boundaries; the current lookups already avoid temporary keys/values.
- No Cow-based rewrite of `detofu_into`: callers already own reusable storage,
  and an intermediate Cow could add allocation or traversal on matches.
- No rewrite of `DetofuMap::detofu`, including its built-in-only case. Its existing
  implementation is retained after measuring width-expanding custom overrides;
  callers can explicitly select `detofu_cow` when borrowing is useful.
- No `String::extend` for the replacement suffix: repeat runs exposed a substantial
  custom-map regression. Normalization uses explicit pushes; DeTofu reuses its
  existing append writer after the first replacement.
- No regex punctuation optimization: the legacy regex helper is not on the
  conversion path. Current dictionary-based punctuation behavior is retained.

