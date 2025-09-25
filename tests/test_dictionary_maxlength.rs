#[cfg(test)]
mod tests {
    use opencc_fmmseg::dictionary_lib::{DictMaxLen, DictionaryMaxlength};
    use std::fs;
    use std::io::{Cursor};
    use std::path::Path;

    // Minimal file-based loader mirroring `from_zstd()` (no crate-specific error type needed here)
    fn load_from_zstd_file<P: AsRef<Path>>(p: P) -> Result<DictionaryMaxlength, Box<dyn std::error::Error>> {
        let bytes = fs::read(p)?;
        let decompressed = zstd::stream::decode_all(Cursor::new(bytes))?;
        let dicts: DictionaryMaxlength = serde_cbor::from_slice(&decompressed)?;
        Ok(dicts)
    }

    // Minimal file-based saver: DictionaryMaxlength -> CBOR -> Zstd
    fn save_to_zstd_file<P: AsRef<Path>>(dicts: &DictionaryMaxlength, p: P) -> Result<(), Box<dyn std::error::Error>> {
        let cbor = serde_cbor::to_vec(dicts)?;
        let compressed = zstd::stream::encode_all(Cursor::new(cbor), 3)?; // level 3 is usually fine
        fs::write(p, compressed)?;
        Ok(())
    }

    // Helper to collect all DictMaxLen refs (keeps count logic in one place)
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

    #[test]
    #[ignore]
    fn roundtrip_zstd_from_disk_and_count() -> Result<(), Box<dyn std::error::Error>> {
        // 1) Write the embedded blob to a temp file to simulate an on-disk source
        let embedded: &[u8] = include_bytes!("dicts/dictionary_maxlength.zstd");
        let tmp_dir = std::env::temp_dir();
        let src_path = tmp_dir.join(format!("opencc_dict_src_{}.zstd", std::process::id()));
        let dst_path = tmp_dir.join(format!("opencc_dict_copy_{}.zstd", std::process::id()));
        fs::write(&src_path, embedded)?;

        // 2) Load DictionaryMaxlength from DISK (file-based zstd)
        let dicts_from_disk = load_from_zstd_file(&src_path)?;
        let dicts_from_disks = DictionaryMaxlength::from_dicts().unwrap_or(DictionaryMaxlength::default());

        // 3) Save to a DIFFERENT compressed zstd filename
        save_to_zstd_file(&dicts_from_disks, &dst_path)?;

        // 4) Load the just-saved compressed file
        let dicts_roundtrip = load_from_zstd_file(&dst_path)?;

        // 5) Count total DictMaxLen tables and how many are non-empty (map not empty)
        let count = |d: &DictionaryMaxlength| {
            let all = all_dicts(d);
            let total = all.len();
            let non_empty = all.iter().filter(|x| !x.map.is_empty()).count();
            (total, non_empty)
        };

        let (t1, n1) = count(&dicts_from_disk);
        let (t3, n3) = count(&dicts_from_disks);
        let (t2, n2) = count(&dicts_roundtrip);

        println!("[from_disk] total DictMaxLen = {t1}, non_empty = {n1}");
        println!("[from_disks] total DictMaxLen = {t3}, non_empty = {n3}");
        println!("[roundtrip] total DictMaxLen = {t2}, non_empty = {n2}");

        // The counts should match after round-trip
        assert_eq!(t1, t2, "total DictMaxLen count mismatched after round-trip");
        assert_eq!(t3, t3, "total DictMaxLen count mismatched after round-trip");
        assert_eq!(n1, n2, "non-empty DictMaxLen count mismatched after round-trip");

        // Cleanup temp files (best-effort)
        let _ = fs::remove_file(&src_path);
        let _ = fs::remove_file(&dst_path);
        Ok(())
    }
}
