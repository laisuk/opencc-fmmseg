use criterion::{criterion_group, criterion_main, Criterion};
use once_cell::sync::Lazy;
use opencc_fmmseg::OpenCC;
use std::fs;
use std::time::Duration;

// Read whole files once and leak into 'static so our slices can be &'static str.
static SIMP_TEXT: Lazy<&'static str> = Lazy::new(|| {
    let s = fs::read_to_string("benches/ChangFeng_Simp.txt")
        .expect("Failed to read benches/ChangFeng_Simp.txt");
    Box::leak(s.into_boxed_str())
});

static TRAD_TEXT: Lazy<&'static str> = Lazy::new(|| {
    let s = fs::read_to_string("benches/ChangFeng_Trad.txt")
        .expect("Failed to read benches/ChangFeng_Trad.txt");
    Box::leak(s.into_boxed_str())
});

// Shared OpenCC instance
static OPENCC: Lazy<OpenCC> = Lazy::new(OpenCC::new);

// Return a char-accurate prefix as &str without allocating.
// If n >= len(s in chars), just return the whole string.
#[inline]
fn char_prefix(s: &'static str, n: usize) -> &'static str {
    // Fast path: if already short enough, return s
    // (computing chars().count() would walk the string; instead, try nth first)
    if let Some((idx, _)) = s.char_indices().nth(n) {
        &s[..idx]
    } else {
        s
    }
}

// Generate input slices once (no copies).
static INPUTS_SIMP: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let s = *SIMP_TEXT;
    vec![
        ("s2t_100",   char_prefix(s, 100)),
        ("s2t_1k",    char_prefix(s, 1_000)),
        ("s2t_10k",   char_prefix(s, 10_000)),
        ("s2t_100k",  char_prefix(s, 100_000)),
        ("s2t_1m",    char_prefix(s, 1_000_000)),
    ]
});

static INPUTS_TRAD: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let s = *TRAD_TEXT;
    vec![
        ("t2s_100",   char_prefix(s, 100)),
        ("t2s_1k",    char_prefix(s, 1_000)),
        ("t2s_10k",   char_prefix(s, 10_000)),
        ("t2s_100k",  char_prefix(s, 100_000)),
        ("t2s_1m",    char_prefix(s, 1_000_000)),
    ]
});

fn bench_convert(c: &mut Criterion) {
    for (name, input) in INPUTS_SIMP.iter() {
        c.bench_function(*name, |b| {
            b.iter(|| {
                OPENCC.convert(input, "s2t", false);
            });
        });
    }

    for (name, input) in INPUTS_TRAD.iter() {
        c.bench_function(*name, |b| {
            b.iter(|| {
                OPENCC.convert(input, "t2s", false);
            });
        });
    }
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = bench_convert
}
criterion_main!(benches);
