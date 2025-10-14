#[cfg(test)]
mod tests {
    use opencc_fmmseg::dictionary_lib::{DictMaxLen, DictionaryMaxlength};
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    // ---------- I/O (internal CBOR only) ----------

    /// Load internal DictionaryMaxlength from .zstd containing *internal* CBOR.
    fn load_from_zstd_file<P: AsRef<Path>>(p: P) -> TestResult<DictionaryMaxlength> {
        let bytes = fs::read(p)?;
        let decompressed = zstd::stream::decode_all(Cursor::new(bytes))?;
        let dicts: DictionaryMaxlength = serde_cbor::from_slice(&decompressed)?;
        Ok(dicts.finish())
    }

    /// Save *internal* DictionaryMaxlength as internal CBOR + zstd.
    fn save_to_zstd_file<P: AsRef<Path>>(dicts: &DictionaryMaxlength, p: P) -> TestResult<()> {
        let cbor = serde_cbor::to_vec(dicts)?;
        let compressed = zstd::stream::encode_all(Cursor::new(cbor), 3)?; // zstd level 3
        fs::write(p, compressed)?;
        Ok(())
    }

    // ---------- Utilities ----------

    /// Fixed order view over the 18 DictMaxLen tables.
    fn all_dicts(d: &DictionaryMaxlength) -> [&DictMaxLen; 18] {
        [
            &d.st_characters, &d.st_phrases,
            &d.ts_characters, &d.ts_phrases,
            &d.tw_phrases, &d.tw_phrases_rev,
            &d.tw_variants, &d.tw_variants_rev, &d.tw_variants_rev_phrases,
            &d.hk_variants, &d.hk_variants_rev, &d.hk_variants_rev_phrases,
            &d.jps_characters, &d.jps_phrases,
            &d.jp_variants, &d.jp_variants_rev,
            &d.st_punctuations, &d.ts_punctuations,
        ]
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DictStats {
        pairs: usize,
        min_len: usize,
        max_len: usize,
        mask: u64,
        non_empty: bool,
    }

    fn collect_stats(d: &DictionaryMaxlength) -> Vec<DictStats> {
        all_dicts(d)
            .iter()
            .map(|x| DictStats {
                pairs: x.map.len(),
                min_len: x.min_len,
                max_len: x.max_len,
                mask: x.key_length_mask,
                non_empty: !x.map.is_empty(),
            })
            .collect()
    }

    /// Invariants that should hold after `.finish()`.
    fn check_invariants(d: &DictionaryMaxlength) {
        for (i, dm) in all_dicts(d).iter().enumerate() {
            assert!(
                dm.min_len <= dm.max_len,
                "Dict[{i}]: min_len {} > max_len {}", dm.min_len, dm.max_len
            );
            // If mask is present (and within representable range), boundaries must be set.
            if dm.key_length_mask != 0 {
                if (1..=64).contains(&dm.min_len) {
                    assert!(
                        dm.has_key_len(dm.min_len),
                        "Dict[{i}]: mask missing min_len {}",
                        dm.min_len
                    );
                }
                if (1..=64).contains(&dm.max_len) {
                    assert!(
                        dm.has_key_len(dm.max_len),
                        "Dict[{i}]: mask missing max_len {}",
                        dm.max_len
                    );
                }
            }
        }
    }

    // ---------- The test ----------

    #[test]
    #[ignore] // large asset; run with: cargo test -- --ignored
    fn roundtrip_internal_cbor_zstd() -> TestResult<()> {
        // 1) Write embedded blob to temp (simulates on-disk source)
        let embedded: &[u8] = include_bytes!("dicts/dictionary_maxlength.zstd");
        let tmp = std::env::temp_dir();
        let src = tmp.join(format!("opencc_src_{}.zstd", std::process::id()));
        let dst = tmp.join(format!("opencc_rt_{}.zstd", std::process::id()));
        fs::write(&src, embedded)?;

        // 2) Load from disk (internal CBOR)
        let disk = load_from_zstd_file(&src)?;

        // 3) Round-trip: save → load
        save_to_zstd_file(&disk, &dst)?;
        let rt = load_from_zstd_file(&dst)?;

        // 4) Quick invariants
        check_invariants(&disk);
        check_invariants(&rt);

        // 5) Compare structural stats per-dictionary
        let s_disk = collect_stats(&disk);
        let s_rt = collect_stats(&rt);

        // totals & non-empty counts
        let tot = |v: &[DictStats]| (v.len(), v.iter().filter(|s| s.non_empty).count());
        let (t_disk, n_disk) = tot(&s_disk);
        let (t_rt, n_rt) = tot(&s_rt);

        println!("[disk     ] total={}, non_empty={}", t_disk, n_disk);
        println!("[roundtrip] total={}, non_empty={}", t_rt, n_rt);

        assert_eq!(t_disk, t_rt, "total DictMaxLen count mismatch");
        assert_eq!(n_disk, n_rt, "non-empty DictMaxLen count mismatch");

        // per-slot pair counts should match exactly for internal→internal round-trip
        let pairs_disk: Vec<_> = s_disk.iter().map(|s| s.pairs).collect();
        let pairs_rt:   Vec<_> = s_rt.iter().map(|s| s.pairs).collect();
        assert_eq!(pairs_disk, pairs_rt, "per-dict pair counts mismatch");

        // min/max/mask should also be stable under internal round-trip
        let bounds_disk: Vec<_> = s_disk.iter().map(|s| (s.min_len, s.max_len)).collect();
        let bounds_rt:   Vec<_> = s_rt.iter().map(|s| (s.min_len, s.max_len)).collect();
        assert_eq!(bounds_disk, bounds_rt, "per-dict min/max mismatch");

        let masks_disk: Vec<_> = s_disk.iter().map(|s| s.mask).collect();
        let masks_rt:   Vec<_> = s_rt.iter().map(|s| s.mask).collect();
        assert_eq!(masks_disk, masks_rt, "per-dict key_length_mask mismatch");

        // 6) Cleanup ( the best effort)
        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&dst);
        Ok(())
    }
}
