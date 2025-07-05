use criterion::{criterion_group, criterion_main, Criterion};
use opencc_fmmseg::OpenCC;
use std::time::Duration;

fn bench_convert_s2t_100k(c: &mut Criterion) {
    let input = "汉字转换繁體字簡體字混用的測試文字".repeat(5883); // ~100,011 characters
    let helper = OpenCC::new();

    c.bench_function("convert_s2t_100k", |b| {
        b.iter(|| {
            helper.convert(&input, "s2t", false); // punctuation = false
        });
    });
}

fn bench_convert_t2s_100k(c: &mut Criterion) {
    let input = "漢字轉換繁體字簡體字混用的測試文字".repeat(5883); // ~100,011 characters
    let helper = OpenCC::new();

    c.bench_function("convert_t2s_100k", |b| {
        b.iter(|| {
            helper.convert(&input, "t2s", false); // punctuation = false
        });
    });
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
    targets = bench_convert_s2t_100k, bench_convert_t2s_100k
}
criterion_main!(benches);
