# seal2t versus s2t: diagnostic investigation

Investigated local `master` at `823d57084234452f672651c9c4eba63b2c8b54d2`, 2026-09-22. No production optimization was
implemented. The only implementation-file addition is a five-line, test-only module hook at the end of `src/opencc.rs`;
instrumentation lives in `benches/seal_diagnostic.rs`. Pre-existing workspace changes were left untouched.

**Finding:** first use is dominated by quadratic-like non-BMP union construction. Warm conversion is dominated by doing
two full conversion rounds instead of one. A guarded scalar fast path looks worthwhile for the bundled Seal data, but
must retain a general fallback for custom dictionaries and IDS behavior.

## Method and scope

Windows, Intel Core i5-13400 (10 cores/16 logical processors), Rust 1.75.0, release profile with optimization level 3,
fat LTO and one codegen unit. Conversion benchmarks load dictionaries, build unions, construct strings and check outputs
before timing. Each timing is the median of seven batched wall-clock samples of at least 30 ms, after
calibration/warm-up. Destruction of returned strings is included. Raw logs retain min/max as well as medians; these are
exploratory measurements, not confidence intervals. Separate layers can differ in cache state, allocation behavior and
compiler optimization, so their times are not an additive CPU profile.

This is a local checkout investigation; the remote branch was not fetched. The actual embedded dictionary was inspected
and its five relevant maps were verified equal to the current plaintext dictionaries. Embedded artifact SHA256:
`B8AF29D0DCBFFD39B4E1DC2AD67799067495F0EB3F6968636A8D9F12B03E20C3`.

## Execution paths

Direct `OpenCC::seal2t` (`src/opencc.rs:1825`) builds stack arrays of dictionary references and calls
`union_for(SealCharactersOnly)` and `union_for(SealVariantsRevOnly)`. It invokes `apply_st_punctuation_only_round_3` →
`apply_dicts_2` or `apply_dicts_3` → `clear_last_error` (global mutex) → `DictRefs::new` and optional round builders →
`apply_segment_replace`.

Direct `OpenCC::s2t` (`src/opencc.rs:1075`) selects `union_for(S2T { punct })` and a stack array of two or three
dictionaries → `apply_dicts_1` → the same error reset and `DictRefs` machinery.

Neither direct API parses a configuration or looks up a conversion-plan object. The optional `convert` wrapper parses
its string to `OpenccConfig`; `convert_with_config` matches the enum and calls the direct helper.

`union_for` (`src/dictionary_lib/union_cache.rs:341`) selects a per-dictionary-instance
`OnceLock<Arc<StarterUnion>>`: first access builds; later accesses clone the Arc. `DictRefs` computes the maximum key
length and stores references plus the Arc per round; it does not clone dictionary maps.

Every round calls `segment_replace_with_union` (`src/opencc.rs:383`):

1. Decode UTF-8 into a newly allocated `Vec<char>`.
2. `get_chars_range` allocates delimiter/optional IDS ranges. Delimiters are included at the end of the preceding range.
   The exact sample has two ranges: `你𿒛，` and `𽌠𽴖𾇓𿭖𿛛码18`.
3. Allocate output, then call `convert_by_union_into` for each range. A standalone delimiter is copied directly;
   complete IDS segments are preserved when enabled.
4. At each position, load starter mask/cap from dense BMP tables or **two non-BMP hash maps**. Reject missing starters;
   enumerate viable lengths in descending order with `for_each_len_dec`.
5. Apply dictionary length gates and, for multi-dictionary rounds, dictionary-specific starter gates. Construct one
   borrowed `&[char]` candidate per viable length; probe `FxHashMap<Box<[char]>, Box<str>>` in precedence order. Append
   the first replacement or copy the unmatched scalar.
6. Each later round decodes and segments the preceding round's output again and allocates a new output string.

Candidates are scalar slices, not UTF-8 substrings: there is no repeated UTF-8 boundary search or candidate-string
allocation in this FMM loop.

| API    | punctuation=false                      | punctuation=true                                              |
|--------|----------------------------------------|---------------------------------------------------------------|
| seal2t | R1 SealCharacters; R2 SealVariantsRev  | Same, plus R3 STPunctuations / StPunctOnly                    |
| s2t    | R1 STPhrases → STCharacters, S2T union | R1 STPhrases → STCharacters → STPunctuations, S2T punct union |

## Dictionary characteristics

Lengths are Unicode scalar counts. BMP columns classify complete keys; starters are listed separately.

| Dictionary      | Entries | min/max | key_length_mask | Single / multi | BMP / non-BMP keys | BMP / non-BMP starters |
|-----------------|--------:|--------:|----------------:|---------------:|-------------------:|-----------------------:|
| SealCharacters  |  11,328 |     1/1 |             0x1 |     11,328 / 0 |         0 / 11,328 |             0 / 11,328 |
| SealVariantsRev |   1,002 |     1/1 |             0x1 |      1,002 / 0 |          296 / 706 |              296 / 706 |
| STPhrases       |  50,887 |    2/12 |           0xffe |     0 / 50,887 |        50,874 / 13 |              3,595 / 4 |
| STCharacters    |   4,014 |     1/1 |             0x1 |      4,014 / 0 |      2,872 / 1,142 |          2,872 / 1,142 |
| STPunctuations  |       4 |     1/1 |             0x1 |          4 / 0 |              4 / 0 |                  4 / 0 |

STPhrases length distribution: `2:20060, 3:14325, 4:13410, 5:1457, 6:789, 7:365, 8:273, 9:69, 10:96, 11:22, 12:21`. Both
Seal maps also have exclusively single-scalar **values**. None of these five maps has delimiter-containing keys.

The Seal unions therefore contain only mask=1/cap=1 active entries. The combined S2T union has 5,102 active BMP and
1,142 active non-BMP starters. Every union allocates 589,824 bytes of dense BMP mask/cap tables, even SealCharacters,
whose BMP tables are entirely unused, plus two sparse astral maps. Per-dictionary metadata also includes sparse starter
masks and dense BMP accelerators.

## Cold behavior: largest first-use cost

Dictionary/OpenCC construction took 33 ms in the initial run and is **excluded** below. These are cold union slots with
loaded dictionaries, not a claim about cold hardware caches or process startup.

| Work                                                 |     Median |
|------------------------------------------------------|-----------:|
| First complete seal2t on a fresh instance (3 trials) | 129.695 ms |
| First complete s2t on a fresh instance (3 trials)    |   4.963 ms |
| Rebuild SealCharacters union (5 trials)              | 129.460 ms |
| Its isolated astral-cap scan expression              | 122.455 ms |
| Rebuild SealVariantsRev union (5 trials)             |   0.671 ms |
| Its isolated astral-cap scan expression              |   0.629 ms |
| Rebuild S2T union (5 trials)                         |   4.844 ms |
| Its isolated astral-cap scan expression              |   4.453 ms |

At `src/dictionary_lib/starter_union.rs:179`, each non-BMP starter scans **all keys** to derive its cap. SealCharacters
executes `11,328 × 11,328 = 128,323,584` key visits. SealVariantsRev executes 707,412; S2T executes 4,787,536. The
isolated Seal scan takes about 95% of the full builder time in separate measurements. This strongly identifies the cold
bottleneck; it is not inferred solely from source. The first public-call medians differ by about 26×.

## Warm sequential results

Punctuation and IDS preservation are disabled. All times below are **microseconds**, including the million-scalar rows.
Full pipelines are distinguished from the diagnostic first round.

| Input scalars    |     seal2t |        s2t | Seal scalar R1 only | Scalar R1 + existing R2 | seal2t / s2t |
|------------------|-----------:|-----------:|--------------------:|------------------------:|-------------:|
| 11, exact sample |      0.521 |      0.290 |               0.081 |                   0.275 |        1.80× |
| 110              |      2.639 |      1.614 |               0.486 |                   1.521 |        1.63× |
| 1,001            |     20.346 |     11.775 |               4.481 |                  13.185 |        1.73× |
| 10,010           |    211.690 |    123.964 |              40.720 |                 124.548 |        1.71× |
| 100,001          |  2,188.631 |  1,429.733 |             447.806 |               1,282.796 |        1.53× |
| 1,000,010        | 24,869.950 | 13,264.033 |           4,899.575 |              14,615.900 |        1.87× |

The sample is 11 scalars / 35 bytes. Repeat counts round upward to whole samples. A repeated corpus stays hot in a small
subset of the dictionaries and is not representative of arbitrary Chinese prose.

### Comparable isolated layers

| Layer                                                       | Exact sample, ns | 1,000,010 scalars, ms |
|-------------------------------------------------------------|-----------------:|----------------------:|
| Seal R1 segment/FMM with built union                        |           264.64 |                13.834 |
| Seal R2 on actual R1 output                                 |           219.38 |                 9.001 |
| Both Seal rounds with built unions, no public orchestration |           465.88 |                25.068 |
| S2T round with built union                                  |           268.06 |                13.271 |
| Seal R1 FMM, chars/ranges/output buffer prebuilt            |           108.78 |                 8.825 |
| S2T FMM, chars/ranges/output buffer prebuilt                |           105.18 |                 9.726 |
| Decode UTF-8 to allocated Vec<char>                         |            83.37 |                 2.002 |
| Segment predecoded chars                                    |            41.04 |                 1.368 |
| Seal scalar map probes only, no output                      |            20.67 |                 2.047 |
| Seal union gate loads only, no output                       |            28.88 |                 2.688 |
| Output append from precomputed replacement pieces           |            46.47 |                 3.200 |
| Allocate/copy completed Seal R1 output                      |            28.31 |                 0.451 |

The lookup-only loop probes **all** scalars; production probes only gate survivors. The append-only loop reads
preallocated strings, so it is a synthetic isolation, not an exact subtraction from production time. Cache misses were
not measured with hardware counters; no percentage is assigned to cache effects or candidate slicing.

Cached `union_for` retrieval including Arc release: SealCharacters 13.49 ns, SealVariantsRev 12.44 ns, S2T 13.53 ns.
Construct/drop `DictRefs`, including Arc clones: two-round Seal 19.32 ns, S2T 11.31 ns. Error mutex reset: 13.73 ns.
These overlap parts of public orchestration and must not be added together as independent costs.

Enum/string dispatch measurements were 492/529 ns versus 521 ns for the direct Seal helper; their small differences are
sensitive to code layout and measurement noise. They do not explain the gap. The public direct helper never performs
config parsing.

On the exact sample, the additional Seal round costs 219 ns, versus a 232 ns public API gap. At 1M, R2 costs about 9.0
ms versus an 11.6 ms gap. R1 and S2T are similar; prebuilt FMM does not show Seal being intrinsically slower on this
corpus. **The extra round is the largest measured warm contributor.**

With punctuation enabled, exact-sample seal2t/s2t take 0.721/0.269 microseconds; at 1M they take 31.813/13.658 ms.
Seal's additional punctuation pass runs even though the sample has no matching quotation marks.

### Parallel mode, measured separately

16-thread Rayon pool warmed before timing. Small-input rows use the existing serial fallback despite parallel mode being
enabled.

| Scalars   | Actual Rayon work? | seal2t, microseconds | s2t, microseconds |
|-----------|--------------------|---------------------:|------------------:|
| 11        | No                 |                0.567 |             0.296 |
| 110       | No                 |                2.722 |             1.632 |
| 1,001     | No                 |               23.371 |            13.198 |
| 10,010    | Yes                |              211.876 |           108.371 |
| 100,001   | Yes                |              867.877 |           472.669 |
| 1,000,010 | Yes                |           11,922.000 |         6,275.933 |

Each round still decodes and segments serially, converts groups of ranges in parallel, then concatenates output parts.
The 10K Seal input gains essentially nothing; larger inputs benefit. The threshold depends on **range count**, not
simply bytes or scalars: chunk size is `(ranges / (threads * 6)).clamp(128, 2048)`. Here 1K has 92 ranges; 10K has 911.
Chunk outputs initially reserve scalar count as bytes, potentially reallocating for non-ASCII output; this is a
source-based opportunity, not a measured dominant cost.

## Exact-sample work counts

| Stage                        | Segments | Gate-rejected positions | Lengths enumerated | Hypothetical lengths excluded | Dict gates rejected | Map probes | Matches | Scalars copied |
|------------------------------|---------:|------------------------:|-------------------:|------------------------------:|--------------------:|-----------:|--------:|---------------:|
| Seal R1                      |        2 |                       5 |                  6 |                             5 |                   0 |          6 |       6 |              5 |
| Seal R2                      |        2 |                      10 |                  1 |                            10 |                   0 |          1 |       1 |             10 |
| Seal total                   |        4 |                      15 |                  7 |                            15 |                   0 |          7 |       7 |             15 |
| S2T R1                       |        2 |                       9 |                  4 |                            38 |                   4 |          4 |       1 |             10 |
| Optional Seal punctuation R3 |        2 |                      11 |                  0 |                            11 |                   0 |          0 |       0 |             11 |

Every round sees 11 scalar positions. `lengths excluded` counts lengths in the hypothetical range
`1..=min(global_max, remaining)` removed by union mask/cap; they are **not executed failed probes**. Candidate slices
constructed equal enumerated lengths here. S2T with punctuation has identical probes/matches, but seven dictionary-gate
rejections instead of four.

- Input: `你𿒛，𽌠𽴖𾇓𿭖𿛛码18` — 11 scalars, 35 bytes.
- Seal R1: `你好，𡭔篆國際編码18` — 11 scalars, 30 bytes.
- Seal R2/final: `你好，小篆國際編码18` — 11 scalars, 29 bytes.
- S2T final: `你𿒛，𽌠𽴖𾇓𿭖𿛛碼18` — 11 scalars, 35 bytes.

These APIs perform different conversions: seal2t does not also normalize the existing simplified `码`. The comparison
measures work on identical input, not equivalent transformations.

For Seal R1 the gate eliminates five map probes, but requires metadata checks at all 11 positions, including two
astral-map queries for each of the six Seal characters. It does no useful longest-match pruning because only length 1
exists. This explains why eliminating that machinery can win even while performing more dictionary probes.

## Optimization opportunities, in priority order

1. **Fix cold astral cap derivation.** Derive caps in one pass over keys, or use exact mask-derived caps where lengths
   are representable and retain correct handling for long keys. Preserve the existing >64 cap semantics. This targets
   the measured 122 ms scan; no optimized builder was implemented or timed, so an exact speedup is not claimed. Reusing
   an OpenCC instance amortizes the existing cost but does not repair it.
2. **Guarded scalar conversion for scalar-only rounds.** Existing Seal R1 takes 2.82–3.35× the diagnostic scalar-loop
   time across sizes. Replacing only R1 in the benchmark pipeline improves the full pipeline by about 1.54–1.90×
   (without public orchestration in the diagnostic). All 11,328 distinct Seal keys together measured 355 microseconds
   FMM versus 107 microseconds scalar, a 3.31× advantage beyond the tiny repeated working set.
3. **Apply the same eligibility check to SealVariantsRev.** A benchmark-only pipeline with two scalar passes takes 149
   ns for the sample and 10.67 ms at 1M, versus 521 ns and 24.87 ms for the public pipeline. This is an exploratory
   upper opportunity, not a drop-in API benchmark. Fusion might save another pass, but was not implemented or measured
   and needs separate semantic proof.
4. **General pipeline/allocation work and Rayon threshold tuning.** Reusing scalar/range buffers or avoiding repeated
   segmentation could help generic rounds, but must respect transformed delimiters, IDS and multi-scalar replacements.
   The current measurements prioritize this below cold cap scans and scalar specialization. Do not remove FMM or
   prioritize config/Arc micro-optimizations based on this evidence.

A scalar fast path is semantically suitable for these bundled dictionaries with IDS preservation disabled: all keys and
values are single scalars, no keys are delimiters, and equivalence was checked on every Seal key plus all six required
corpus sizes. An unconditional `seal2t` rewrite is **not** safe: custom Seal slots can contain phrases; IDS-preserving
mode must keep complete IDS segments untouched; and custom delimiter mappings must retain current standalone-delimiter
behavior. Eligibility should be derived from the actual immutable dictionary metadata/content, with the current FMM path
retained for ineligible data. One-scalar keys alone do not require one-scalar replacements for an individual scalar
pass, but output shape matters for fusion.

## Reproduction and cleanup

Run sequentially (do not run benchmarks alongside tests):

```powershell
cargo test --release --offline -p opencc-fmmseg --lib seal_diagnostic::investigate -- --ignored --nocapture --test-threads=1
cargo test --release --offline -p opencc-fmmseg --lib seal_diagnostic::cold_and_diverse -- --ignored --nocapture --test-threads=1
cargo test --workspace --release --offline
```

`benches/seal_diagnostic_results.txt` contains warm/layer/count measurements; `benches/seal_diagnostic_cold.txt`
contains repeated cold measurements and embedded/text equivalence checks. `benches/seal_diagnostic_tests.txt` records
the workspace test run. Both diagnostic tests passed separately. `cargo test --workspace --release --offline` exited
successfully: **278 passed, 0 failed, 25 ignored** (the ignored total includes the two manual diagnostic tests).
`rustfmt --check` for the harness and `git diff --check` also passed.

Remove only the final test-module hook from `src/opencc.rs` and the `benches/seal_diagnostic*` files to remove the
investigation. No dependencies, public APIs, configs, dictionary formats or production conversion semantics changed.