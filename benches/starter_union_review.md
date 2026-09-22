# Review of the current StarterUnion cap fix

The user's current mask-derived astral cap implementation is preserved. This follow-up adds four unit tests only; it does not change the production builder, implement a scalar fast path, introduce Seal-specific logic, or rerun the previous benchmark suite.

## Correctness proof

For a populated, consistent DictMaxLen, `set_key_len_bit` records a key length L in bit L-1 precisely when 1 <= L <= 64. Thus for any nonzero per-starter mask M, `max_len_from_mask(M) = 64 - M.leading_zeros()` equals the maximum represented key length. Mixed lengths do not change this identity. Bit 63 means exactly length 64 in the source metadata; its integer result is 64, which fits u8.

When a starter has no key longer than 64, this is exactly the result of the removed full-key scan. If a dictionary has long keys for some other starter, that does not affect this per-starter identity.

For a starter with lengths above 64, the first pass may temporarily underestimate its cap. The unchanged later pass examines every key in each dictionary with `max_len > 64`. For each long key it sets the union's bit 63 (the 64+ bucket) and raises that starter's cap to `max(existing, min(key.len(), 255))`. It handles BMP and astral starters independently. A long-only starter has mask zero and is skipped by the first pass, but the later pass inserts its missing union entries. An exact length-64 key is not required.

Masks merge by OR and caps by maximum in both passes. These operations are associative and commutative, so merging several dictionaries, in either order, cannot erase lengths or lower a repaired cap. The final cap equals the maximum key length for that starter across all dictionaries, subject to the existing u8 saturation at 255. The BMP first-pass logic is unchanged; its existing dense metadata and the long-key pass retain the same final values.

Therefore, the change preserves the final StarterUnion for valid metadata, including long keys. It does not introduce a new 64-character cap. Lengths above 255 remain a pre-existing representation limitation (the normal pair builder also has a debug assertion against such keys).

## Added regressions

In `src/dictionary_lib/starter_union.rs`:

- `build_preserves_long_astral_caps`: an 80-scalar astral-only key with initial mask zero; checks recovered cap and 64+ bit.
- `astral_mask_caps_match_key_scan_through_length_64`: checks every maximum from 1 through 64 against the old scan on a small synthetic dictionary, with mixed short keys and an explicit bit-63 assertion.
- `build_merges_mixed_lengths_and_long_caps_in_either_order`: combines BMP and astral starters across dictionaries with lengths 1, 2, 3, 64, 65, 80 and 255; checks OR/max semantics in both orders and unrelated short starters.
- `long_key_pass_repairs_mixed_starters_without_an_exact_64_key`: uses the public append API to combine short and long BMP/astral keys without length 64, checking both repaired caps and bucket bits.

The existing long-BMP regression remains unchanged.

## Validation

`cargo test --workspace --offline` completed successfully in the normal debug/test profile, including doc tests:

- 219 unit/integration tests passed.
- 63 doc tests passed.
- Total: **282 passed, 0 failed, 25 ignored**. Two ignored tests are the previous manual diagnostic benchmarks; the remaining ignores were already present.

The new test file formatting check and scoped diff whitespace check passed. A whole-working-tree whitespace check reports pre-existing trailing whitespace in `THIRD_PARTY_NOTICES.md:10`; that unrelated file was not edited.

Full output: `benches/starter_union_review_tests.txt`. No cold timing or performance matrix was run.

## Remaining builder work and allocations

There is no remaining per-starter nested dictionary-key scan in the production builder. For dictionaries with maximum length <=64, the builder visits starter metadata only. Dictionaries containing longer keys incur one additional full-map iteration each. Existing metadata cannot supply the exact astral cap above 64, so retaining that generic pass is necessary without adding new metadata.

Strict complexity also includes initializing the two dense BMP arrays (65,536 entries each) and hash-table iteration capacity: broadly O(65,536 + total starter-table traversal + total map traversal for long-key dictionaries), with amortized hash insertion. The existing O(S) documentation omits dense initialization and the long-key pass.

The two BMP arrays occupy 576 KiB even for astral-only unions. This is fixed overhead, not another quadratic problem, and current consumers rely on full BMP-sized arrays for unchecked indexing. The two astral maps involve ordinary amortized growth and separate mask/cap insertions; there is no measured justification for redesigning them in this task. No further production change is recommended for this builder now.

## Separate pre-existing downstream issues to follow up

These are source-review findings outside the changed builder, not regressions introduced by the cap fix. Correct union metadata alone does not establish correct end-to-end matching for every long custom key.

1. `src/utils.rs:66`: the range mask uses `1u64.wrapping_shl(limit).wrapping_sub(1)`. At limit=64, the shift wraps to zero, producing a zero mask. At cap exactly 64 it enumerates no lengths; at cap above 64 it enumerates the >=64 branch but loses shorter fallback candidates. This should receive focused boundary tests and a separate fix.
2. `src/opencc.rs:616` passes bit=63 for all candidate lengths >=64. `DictMaxLen::starter_allows_dict` (`src/dictionary_lib/dict_max_len.rs:924`) consequently takes the BMP exact-bit branch rather than its long-cap branch. The astral branch calls `has_starter_len`, which rejects lengths above 64. Thus multi-dictionary long-key matching has pre-existing gate limitations even with correct union caps.
3. `src/dictionary_lib/dict_max_len.rs:484`: the pair-builder's debug sanity assertion expects a long key's source mask either to contain bit 63 or be zero. That conflicts with valid mixtures such as lengths 1 and 80, because the source mask intentionally records only exact lengths <=64. The append/rebuild path used in the new regression permits that valid mixture. Aligning this assertion with actual source-mask semantics is another small separate correctness follow-up.

Leave these unrelated changes separate from the reviewed cold-start fix. The current fix is generic and correct at the StarterUnion boundary.