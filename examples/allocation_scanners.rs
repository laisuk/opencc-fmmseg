//! Compare scan strategies against the actual private character lookup tables.
#![allow(dead_code)]
#[path = "../src/compat_ideographs.rs"]
mod compat_ideographs;
#[path = "../src/lazy_replace.rs"]
mod lazy_replace;
#[path = "../src/rustc_hash/mod.rs"]
mod rustc_hash;
#[path = "../src/unicode_compat.rs"]
mod unicode_compat;

use std::{borrow::Cow, hint::black_box, time::Instant};

fn prescan(input: &str, map: impl Fn(char) -> char) -> Cow<'_, str> {
    if !input.chars().any(|ch| map(ch) != ch) {
        return Cow::Borrowed(input);
    }
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        output.push(map(ch));
    }
    Cow::Owned(output)
}

fn complete_scan(input: &str, map: impl Fn(char) -> char) -> Cow<'_, str> {
    let changes = input.chars().filter(|&ch| map(ch) != ch).count();
    if changes == 0 {
        return Cow::Borrowed(input);
    }
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        output.push(map(ch));
    }
    Cow::Owned(output)
}

fn measure<T>(label: &str, mut f: impl FnMut() -> T) {
    black_box(f());
    let start = Instant::now();
    black_box(f());
    let iterations = ((0.015 / start.elapsed().as_secs_f64()) as usize).clamp(1, 100_000);
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(f());
        }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    samples.sort_by(f64::total_cmp);
    println!("{label},{:.1}", samples[3]);
}

fn main() {
    let table = unicode_compat::UnicodeCompat::builtin();
    println!("case,median_ns");
    let plain = "普通漢字，故事繼續。".repeat(128);
    for (name, input) in [
        ("ascii", "ASCII text 123. ".repeat(128)),
        ("unchanged", plain.clone()),
        ("first", format!("聼{plain}")),
        ("middle", format!("{plain}聼{plain}")),
        ("last", format!("{plain}聼")),
        ("dense", "金聼".repeat(128)),
        ("short", "普通漢字，故事繼續。".into()),
        (
            "novel_3m",
            "普通漢字，故事繼續。"
                .chars()
                .cycle()
                .take(3_000_000)
                .collect(),
        ),
    ] {
        let map = |ch| table.normalize_all_char(ch);
        let first = lazy_replace::replace_chars(&input, map);
        assert_eq!(first, prescan(&input, map));
        assert_eq!(first, complete_scan(&input, map));
        measure(&format!("first/{name}"), || {
            lazy_replace::replace_chars(black_box(&input), map)
        });
        measure(&format!("prescan/{name}"), || {
            prescan(black_box(&input), map)
        });
        measure(&format!("complete_scan/{name}"), || {
            complete_scan(black_box(&input), map)
        });
    }
}
