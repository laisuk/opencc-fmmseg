use criterion::{criterion_group, criterion_main, Criterion};
use opencc_fmmseg::OpenCC;
use std::time::Duration;
use once_cell::sync::Lazy;

// Base text of length ~80
static BASE_SIMP: &str = "蟹者之王，应该是大闸蟹。市面上能买到的螃蟹，我都试过，也有些不卖的，全部加起来，一比较，就知道没有一种蟹比大闸蟹更香了。\n";
static BASE_TRAD: &str = "蟹者之王，應該是大閘蟹。市面上能買到的螃蟹，我都試過，也有些不賣的，全部加起來，一比較，就知道沒有一種蟹比大閘蟹更香了。\n";

// Shared OpenCC instance
static OPENCC: Lazy<OpenCC> = Lazy::new(OpenCC::new);

// Generate input strings once
static INPUTS_SIMP: Lazy<Vec<(&'static str, String)>> = Lazy::new(|| {
    vec![
        ("s2t_100", BASE_SIMP.repeat(2)),
        ("s2t_1k", BASE_SIMP.repeat(13)),
        ("s2t_10k", BASE_SIMP.repeat(125)),
        ("s2t_100k", BASE_SIMP.repeat(1_250)),
        ("s2t_1m", BASE_SIMP.repeat(12_500)),
    ]
});

static INPUTS_TRAD: Lazy<Vec<(&'static str, String)>> = Lazy::new(|| {
    vec![
        ("t2s_100", BASE_TRAD.repeat(2)),
        ("t2s_1k", BASE_TRAD.repeat(13)),
        ("t2s_10k", BASE_TRAD.repeat(125)),
        ("t2s_100k", BASE_TRAD.repeat(1_250)),
        ("t2s_1m", BASE_TRAD.repeat(12_500)),
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
