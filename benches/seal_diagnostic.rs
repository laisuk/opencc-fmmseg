//! Diagnosis only. Run the ignored test in release mode with --nocapture --test-threads=1.
use super::*;
use std::{
    collections::BTreeMap,
    hint::black_box,
    time::{Duration, Instant},
};
const SAMPLE: &str = "你𿒛，𽌠𽴖𾇓𿭖𿛛码18";
fn bench<T>(name: &str, n: usize, mut f: impl FnMut() -> T) {
    let mut it = 1;
    loop {
        let t = Instant::now();
        for _ in 0..it {
            black_box(f());
        }
        if t.elapsed() >= Duration::from_millis(10) {
            break;
        }
        it *= 2;
    }
    let mut samples = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        let mut count = 0;
        while t.elapsed() < Duration::from_millis(30) {
            for _ in 0..it {
                black_box(f());
            }
            count += it;
        }
        samples.push(t.elapsed().as_nanos() as f64 / count as f64);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "TIME,{name},{n},{:.2},{:.2},{:.2}",
        samples[3], samples[0], samples[6]
    );
}
fn scalar(input: &str, d: &DictMaxLen) -> String {
    let mut out = String::with_capacity(input.len() + (input.len() >> 6));
    for ch in input.chars() {
        if let Some(v) = d.map.get(&[ch][..]) {
            out.push_str(v)
        } else {
            out.push(ch)
        }
    }
    out
}
fn stats(name: &str, d: &DictMaxLen) {
    let mut lengths = BTreeMap::<usize, usize>::new();
    let mut values = BTreeMap::<usize, usize>::new();
    let mut bmp = 0;
    let mut delimiters = 0;
    for (k, v) in &d.map {
        *lengths.entry(k.len()).or_default() += 1;
        *values.entry(v.chars().count()).or_default() += 1;
        if k.iter().all(|c| (*c as u32) <= 0xffff) {
            bmp += 1;
        }
        if k.iter().any(|c| is_delimiter(*c)) {
            delimiters += 1;
        }
    }
    let bs = d
        .starter_len_mask
        .keys()
        .filter(|c| (**c as u32) <= 0xffff)
        .count();
    println!("DICT,{name},entries={},min={},max={},mask={:#x},lengths={lengths:?},value_lengths={values:?},all_bmp_keys={bmp},non_bmp_keys={},bmp_starters={bs},astral_starters={},delimiter_keys={delimiters}",d.len(),d.min_key_len(),d.max_key_len(),d.key_length_mask,d.len()-bmp,d.starter_len_mask.len()-bs);
}
#[derive(Default, Debug)]
struct Counts {
    segments: usize,
    positions: usize,
    gate_rejected_positions: usize,
    lengths_excluded: usize,
    enumerated: usize,
    dict_gate_rejected: usize,
    slices: usize,
    lookups: usize,
    matches: usize,
    copied: usize,
}
// Safe counted mirror; timing always uses the unmodified production implementation.
// lengths_excluded counts hypothetical lengths 1..=global_cap removed by mask/cap.
fn count(cc: &OpenCC, name: &str, input: &str, ds: &[&DictMaxLen], u: &StarterUnion) -> String {
    let chars: Vec<_> = input.chars().collect();
    let ranges = cc.get_chars_range(&chars, true, false);
    let mut c = Counts {
        segments: ranges.len(),
        ..Counts::default()
    };
    let mut out = String::new();
    let max = ds.iter().map(|d| d.max_len).max().unwrap_or(1);
    for r in ranges {
        let text = &chars[r];
        if text.len() == 1 && is_delimiter(text[0]) {
            out.push(text[0]);
            c.copied += 1;
            continue;
        }
        let mut pos = 0;
        while pos < text.len() {
            c.positions += 1;
            let ch = text[pos];
            let (mask, cap) = if (ch as u32) <= 0xffff {
                (u.bmp_mask[ch as usize], u.bmp_cap[ch as usize])
            } else {
                (
                    *u.astral_mask.get(&ch).unwrap_or(&0),
                    *u.astral_cap.get(&ch).unwrap_or(&0),
                )
            };
            let global = max.min(text.len() - pos);
            let cap_here = global.min(cap as usize);
            let mut viable = 0;
            for_each_len_dec(mask, cap_here, |_| {
                viable += 1;
                false
            });
            c.lengths_excluded += global - viable;
            if mask == 0 || cap == 0 {
                c.gate_rejected_positions += 1;
                c.copied += 1;
                out.push(ch);
                pos += 1;
                continue;
            }
            let mut matched = false;
            for_each_len_dec(mask, cap_here, |len| {
                c.enumerated += 1;
                let bit = if len >= 64 { 63 } else { len - 1 };
                let mut sliced = false;
                for &d in ds {
                    if !d.has_key_len(len) || (ds.len() > 1 && !d.starter_allows_dict(ch, len, bit))
                    {
                        c.dict_gate_rejected += 1;
                        continue;
                    }
                    if !sliced {
                        c.slices += 1;
                        sliced = true;
                    }
                    c.lookups += 1;
                    if let Some(v) = d.map.get(&text[pos..pos + len]) {
                        c.matches += 1;
                        out.push_str(v);
                        pos += len;
                        matched = true;
                        return true;
                    }
                }
                false
            });
            if !matched {
                c.copied += 1;
                out.push(ch);
                pos += 1;
            }
        }
    }
    assert!(c.segments > 0);
    assert_eq!(out, cc.segment_replace_with_union(input, ds, max, u));
    println!("COUNT,{name},input_scalars={},input_bytes={},output_scalars={},output_bytes={},counts={c:?},output={out}",chars.len(),input.len(),out.chars().count(),out.len());
    out
}
#[test]
#[ignore = "manual release-only investigation"]
fn investigate() {
    assert!(!cfg!(debug_assertions), "use --release");
    println!(
        "META,threads={},sample_scalars={},sample_bytes={}",
        rayon::current_num_threads(),
        SAMPLE.chars().count(),
        SAMPLE.len()
    );
    let t = Instant::now();
    let mut cc = OpenCC::new();
    println!("INIT,opencc_ns={}", t.elapsed().as_nanos());
    cc.set_parallel(false);
    let d = &cc.dictionary;
    let seal = [&d.seal_characters];
    let variants = [&d.seal_variants_rev];
    let st = [&d.st_phrases, &d.st_characters];
    let punct = [&d.st_punctuations];
    for (name, dict) in [
        ("SealCharacters", seal[0]),
        ("SealVariantsRev", variants[0]),
        ("STPhrases", st[0]),
        ("STCharacters", st[1]),
        ("STPunctuations", punct[0]),
    ] {
        stats(name, dict);
    }
    let t = Instant::now();
    let us = d.union_for(UnionKey::SealCharactersOnly);
    println!("COLD,SealCharacters_union_ns={}", t.elapsed().as_nanos());
    let t = Instant::now();
    let uv = d.union_for(UnionKey::SealVariantsRevOnly);
    println!("COLD,SealVariantsRev_union_ns={}", t.elapsed().as_nanos());
    let t = Instant::now();
    let ut = d.union_for(UnionKey::S2T { punct: false });
    println!("COLD,S2T_union_ns={}", t.elapsed().as_nanos());
    let up = d.union_for(UnionKey::StPunctOnly);
    for (name, u) in [
        ("SealCharacters", &us),
        ("SealVariantsRev", &uv),
        ("S2T", &ut),
    ] {
        println!(
            "UNION,{name},bmp_active={},astral={},bmp_table_bytes={}",
            u.bmp_mask.iter().filter(|m| **m != 0).count(),
            u.astral_mask.len(),
            u.bmp_mask.len() * 8 + u.bmp_cap.len()
        );
    }
    assert_eq!(seal[0].max_len, 1);
    assert!(!seal[0]
        .map
        .keys()
        .any(|k| k.iter().any(|ch| is_delimiter(*ch))));
    let all_keys: String = seal[0].map.keys().map(|k| k[0]).collect();
    assert_eq!(
        scalar(&all_keys, seal[0]),
        cc.segment_replace_with_union(&all_keys, &seal, 1, &us)
    );
    println!("CHECK,all_seal_keys={}", seal[0].len());
    let intermediate = count(&cc, "seal_round1", SAMPLE, &seal, &us);
    let final_seal = count(&cc, "seal_round2", &intermediate, &variants, &uv);
    count(&cc, "seal_punct_round3", &final_seal, &punct, &up);
    count(&cc, "s2t_round1", SAMPLE, &st, &ut);
    let stp = [&d.st_phrases, &d.st_characters, &d.st_punctuations];
    let utp = d.union_for(UnionKey::S2T { punct: true });
    count(&cc, "s2t_punct_round1", SAMPLE, &stp, &utp);
    bench("cached_union_seal", 11, || {
        d.union_for(black_box(UnionKey::SealCharactersOnly))
    });
    bench("cached_union_variants", 11, || {
        d.union_for(black_box(UnionKey::SealVariantsRevOnly))
    });
    bench("cached_union_s2t", 11, || {
        d.union_for(black_box(UnionKey::S2T { punct: false }))
    });
    bench("dictrefs_seal_construct", 11, || {
        DictRefs::new(black_box(&seal), us.clone()).with_round_2(black_box(&variants), uv.clone())
    });
    bench("dictrefs_s2t_construct", 11, || {
        DictRefs::new(black_box(&st), ut.clone())
    });
    bench("clear_last_error", 11, OpenCC::clear_last_error);
    for target in [11usize, 100, 1000, 10_000, 100_000, 1_000_000] {
        let input = SAMPLE.repeat((target + 10) / 11);
        let n = input.chars().count();
        let mid = cc.segment_replace_with_union(&input, &seal, 1, &us);
        let expected = cc.seal2t(&input, false);
        let direct = scalar(&input, seal[0]);
        assert_eq!(direct, mid);
        assert_eq!(
            cc.segment_replace_with_union(&direct, &variants, variants[0].max_len, &uv),
            expected
        );
        println!(
            "INPUT,scalars={n},bytes={},mid_bytes={}",
            input.len(),
            mid.len()
        );
        bench("seal2t_seq", n, || cc.seal2t(black_box(&input), false));
        bench("s2t_seq", n, || cc.s2t(black_box(&input), false));
        bench("seal2t_punct_seq", n, || cc.seal2t(black_box(&input), true));
        bench("s2t_punct_seq", n, || cc.s2t(black_box(&input), true));
        bench("seal_rounds_prebuilt", n, || {
            let mid = cc.segment_replace_with_union(black_box(&input), &seal, 1, &us);
            cc.segment_replace_with_union(&mid, &variants, variants[0].max_len, &uv)
        });
        bench("seal_round1", n, || {
            cc.segment_replace_with_union(black_box(&input), &seal, 1, &us)
        });
        bench("seal_round2", n, || {
            cc.segment_replace_with_union(black_box(&mid), &variants, variants[0].max_len, &uv)
        });
        bench("s2t_round_prebuilt", n, || {
            cc.segment_replace_with_union(
                black_box(&input),
                &st,
                st[0].max_len.max(st[1].max_len),
                &ut,
            )
        });
        bench("seal_scalar_round1", n, || {
            scalar(black_box(&input), seal[0])
        });
        bench("seal_scalar_plus_round2", n, || {
            let mid = scalar(black_box(&input), seal[0]);
            cc.segment_replace_with_union(&mid, &variants, variants[0].max_len, &uv)
        });
        let chars: Vec<_> = input.chars().collect();
        let ranges = cc.get_chars_range(&chars, true, false);
        let mut out = String::with_capacity(input.len() * 2);
        bench("utf8_decode_vec", n, || {
            black_box(&input).chars().collect::<Vec<_>>()
        });
        bench("segmentation", n, || {
            cc.get_chars_range(black_box(&chars), true, false)
        });
        bench("seal_fmm_preallocated", n, || {
            out.clear();
            for r in &ranges {
                cc.convert_by_union_into(black_box(&chars[r.clone()]), &seal, 1, &us, &mut out);
            }
            black_box(&out);
            out.len()
        });
        bench("s2t_fmm_preallocated", n, || {
            out.clear();
            for r in &ranges {
                cc.convert_by_union_into(
                    black_box(&chars[r.clone()]),
                    &st,
                    st[0].max_len.max(st[1].max_len),
                    &ut,
                    &mut out,
                );
            }
            black_box(&out);
            out.len()
        });
        bench("seal_scalar_lookups_no_output", n, || {
            for ch in black_box(&chars) {
                black_box(seal[0].map.get(std::slice::from_ref(ch)));
            }
        });
        bench("seal_union_gate_no_output", n, || {
            for ch in black_box(&chars) {
                if (*ch as u32) <= 0xffff {
                    black_box((us.bmp_mask[*ch as usize], us.bmp_cap[*ch as usize]));
                } else {
                    black_box((us.astral_mask.get(ch), us.astral_cap.get(ch)));
                }
            }
        });
        let pieces: Vec<String> = chars
            .iter()
            .map(|ch| {
                seal[0]
                    .get(&[*ch])
                    .map(str::to_owned)
                    .unwrap_or_else(|| ch.to_string())
            })
            .collect();
        bench("seal_output_append_precomputed", n, || {
            let mut s = String::with_capacity(input.len() + (input.len() >> 6));
            for p in black_box(&pieces) {
                s.push_str(p);
            }
            s
        });
        bench("output_copy", n, || black_box(&mid).to_owned());
        if target == 11 {
            bench("seal_config_enum", n, || {
                cc.convert_with_config(black_box(&input), OpenccConfig::Seal2t, false)
            });
            bench("seal_config_string", n, || {
                cc.convert(black_box(&input), "seal2t", false)
            });
        }
    }
    cc.set_parallel(true);
    for target in [11usize, 100, 1000, 10_000, 100_000, 1_000_000] {
        let input = SAMPLE.repeat((target + 10) / 11);
        let n = input.chars().count();
        let chars: Vec<_> = input.chars().collect();
        let ranges = cc.get_chars_range(&chars, true, false);
        let chunk = (ranges.len() / (rayon::current_num_threads() * 6))
            .max(128)
            .min(2048);
        println!(
            "PARALLEL,scalars={n},ranges={},chunk={chunk},uses_rayon={}",
            ranges.len(),
            ranges.len() > chunk
        );
        bench("seal2t_parallel_enabled", n, || {
            cc.seal2t(black_box(&input), false)
        });
        bench("s2t_parallel_enabled", n, || {
            cc.s2t(black_box(&input), false)
        });
    }
}
#[test]
#[ignore = "manual cold-build and dictionary-diversity diagnosis"]
fn cold_and_diverse() {
    assert!(!cfg!(debug_assertions));
    let mut cc = OpenCC::new();
    cc.set_parallel(false);
    let d = &cc.dictionary;
    let text = DictionaryMaxlength::from_dicts_at("dicts").unwrap();
    for (name, a, b) in [
        ("SealCharacters", &d.seal_characters, &text.seal_characters),
        (
            "SealVariantsRev",
            &d.seal_variants_rev,
            &text.seal_variants_rev,
        ),
        ("STPhrases", &d.st_phrases, &text.st_phrases),
        ("STCharacters", &d.st_characters, &text.st_characters),
        ("STPunctuations", &d.st_punctuations, &text.st_punctuations),
    ] {
        assert_eq!(a.map, b.map, "embedded vs text: {name}");
        println!("CHECK,embedded_matches_text,{name}");
    }
    for (name, ds) in [
        ("SealCharacters", vec![&d.seal_characters]),
        ("SealVariantsRev", vec![&d.seal_variants_rev]),
        ("S2T", vec![&d.st_phrases, &d.st_characters]),
    ] {
        let visits: usize = ds
            .iter()
            .map(|d| {
                d.starter_len_mask
                    .keys()
                    .filter(|c| (**c as u32) > 0xffff)
                    .count()
                    * d.len()
            })
            .sum();
        println!("CAP_SCAN,{name},key_visits={visits}");
        for trial in 0..5 {
            let t = Instant::now();
            let u = StarterUnion::build(black_box(&ds));
            let ns = t.elapsed().as_nanos();
            black_box(&u);
            println!("BUILD,{name},{trial},{ns}");
        }
        // Isolate the exact repeated-key-scan expression from StarterUnion::build.
        bench(&format!("{name}_astral_cap_scan"), visits, || {
            let mut sum = 0usize;
            for d in black_box(&ds) {
                for &ch in d.starter_len_mask.keys().filter(|c| (**c as u32) > 0xffff) {
                    let cap = d
                        .map
                        .keys()
                        .filter(|k| k.first().copied() == Some(ch))
                        .map(|k| u8::try_from(k.len()).unwrap_or(u8::MAX))
                        .max()
                        .unwrap_or(0);
                    sum += cap as usize;
                }
            }
            black_box(sum)
        });
    }
    let seal = [&d.seal_characters];
    let u = d.union_for(UnionKey::SealCharactersOnly);
    let mut keys: Vec<_> = seal[0].map.keys().map(|k| k[0]).collect();
    keys.sort_unstable();
    let input: String = keys.iter().collect();
    assert_eq!(
        scalar(&input, seal[0]),
        cc.segment_replace_with_union(&input, &seal, 1, &u)
    );
    bench("diverse_seal_fmm_round", keys.len(), || {
        cc.segment_replace_with_union(black_box(&input), &seal, 1, &u)
    });
    bench("diverse_seal_scalar_round", keys.len(), || {
        scalar(black_box(&input), seal[0])
    });
    // The second scalar round is measured only as an additional diagnostic, never
    // substituted into production; all sample-size outputs are checked first.
    assert_eq!(d.seal_variants_rev.max_len, 1);
    for target in [11usize, 100, 1000, 10_000, 100_000, 1_000_000] {
        let input = SAMPLE.repeat((target + 10) / 11);
        let n = input.chars().count();
        assert_eq!(
            scalar(&scalar(&input, seal[0]), &d.seal_variants_rev),
            cc.seal2t(&input, false)
        );
        bench("seal_two_scalar_rounds", n, || {
            scalar(&scalar(black_box(&input), seal[0]), &d.seal_variants_rev)
        });
    }
    // Cold public calls on fresh instances, with dictionary loading outside timer.
    for trial in 0..3 {
        for mode in ["seal2t", "s2t"] {
            let mut fresh = OpenCC::new();
            fresh.set_parallel(false);
            let t = Instant::now();
            let out = if mode == "seal2t" {
                fresh.seal2t(SAMPLE, false)
            } else {
                fresh.s2t(SAMPLE, false)
            };
            let ns = t.elapsed().as_nanos();
            black_box(out);
            println!("COLD_API,{mode},{trial},{ns}");
        }
    }
}
