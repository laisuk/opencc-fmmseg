//! Dependency-free diagnostic: cargo +stable run --release --example allocation_review
#![allow(dead_code)]
#[path = "../tools/opencc-rs/src/text_converter.rs"]
mod text_converter;

use opencc_fmmseg::{DetofuLevel, DetofuMap, OpenCC};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
use text_converter::{create_text_converter, NormalizationMode, TextConverterOptions};

struct CountingAllocator;
static COUNT: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNT.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(size, Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure<T>(name: &str, input: &str, mut f: impl FnMut() -> T) {
    if let Ok(filter) = std::env::var("ALLOCATION_CASE") {
        if !name.contains(&filter) {
            return;
        }
    }
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
    let mut allocations = Vec::new();
    for _ in 0..7 {
        ALLOCS.store(0, Relaxed);
        BYTES.store(0, Relaxed);
        COUNT.store(true, Relaxed);
        black_box(f());
        COUNT.store(false, Relaxed);
        allocations.push((ALLOCS.load(Relaxed), BYTES.load(Relaxed)));
    }
    allocations.sort();
    println!(
        "{name},{},{},{:.1},{},{}",
        input.chars().count(),
        input.len(),
        samples[3],
        allocations[3].0,
        allocations[3].1
    );
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--cold") {
        // One observation per fresh process; repeat the executable seven times.
        let start = Instant::now();
        let cc = OpenCC::new();
        println!("cold_opencc_ns,{}", start.elapsed().as_nanos());
        let start = Instant::now();
        black_box(cc.normalize_compat("金"));
        println!("cold_compat_ns,{}", start.elapsed().as_nanos());
        let start = Instant::now();
        black_box(cc.normalize_compat_extended("聼"));
        println!("cold_extended_ns,{}", start.elapsed().as_nanos());
        let start = Instant::now();
        black_box(cc.detofu("𬴂", DetofuLevel::ExtB));
        println!("cold_detofu_ns,{}", start.elapsed().as_nanos());
        return;
    }
    let mut cc = OpenCC::new();
    // Deterministic serial attribution. Also measure the default parallel path below.
    cc.set_parallel(false);
    let map = DetofuMap::builtin(DetofuLevel::ExtB);
    let custom = map.clone().with_custom_pairs(&[('𬴂', '馬'), ('A', '𠀀')]);
    println!("case,chars,input_bytes,median_ns,allocations,allocated_bytes");
    let mut cases = vec![
        ("empty", String::new()),
        ("ascii", "The quick brown fox 123. ".repeat(128)),
        (
            "simplified",
            "这是普通汉字，小说里的故事继续发展。".repeat(128),
        ),
        (
            "traditional",
            "這是普通漢字，小說裡的故事繼續發展。".repeat(128),
        ),
        (
            "rare_compat",
            format!(
                "{}金聼{}",
                "普通漢字，".repeat(128),
                "故事繼續。".repeat(128)
            ),
        ),
        ("mixed", "OpenCC 汉字與小說 ABC 123，測試。".repeat(128)),
        ("supplementary", "𠀀𠀁𠀂普通文字。".repeat(128)),
        (
            "occasional_tofu",
            format!("{}𬴂{}", "普通漢字，".repeat(128), "故事繼續。".repeat(128)),
        ),
        ("short", "“汉字转换”，小說與故事。".into()),
        (
            "novel_3m",
            "这是普通汉字，小说里的故事继续发展。\n"
                .chars()
                .cycle()
                .take(3_000_000)
                .collect(),
        ),
    ];
    // Optional real novel, bounded to approximately 3 million scalar values.
    if let Some(path) = args.get(1) {
        let novel = std::fs::read_to_string(path).expect("UTF-8 novel");
        cases.push(("real_novel", novel.chars().take(3_000_000).collect()));
    }
    for (name, text) in &cases {
        measure(&format!("compat/{name}"), text, || {
            cc.normalize_compat(black_box(text))
        });
        measure(&format!("extended/{name}"), text, || {
            cc.normalize_compat_extended(black_box(text))
        });
        measure(&format!("unicode/{name}"), text, || {
            cc.normalize_unicode_compat(black_box(text))
        });
        measure(&format!("detofu/{name}"), text, || {
            map.detofu(black_box(text))
        });
        measure(&format!("custom/{name}"), text, || {
            custom.detofu(black_box(text))
        });
        let mut output = String::with_capacity(text.len() * 2);
        measure(&format!("into/{name}"), text, || {
            output.clear();
            map.detofu_into(black_box(text), &mut output);
            black_box(&output);
        });
        measure(&format!("convert/{name}"), text, || {
            cc.convert(black_box(text), "s2t", true)
        });
        for (mode_name, normalization) in [
            ("none", NormalizationMode::None),
            ("compat", NormalizationMode::Compat),
            ("extended", NormalizationMode::CompatExtended),
        ] {
            let pipeline = create_text_converter(
                &cc,
                TextConverterOptions {
                    config: "s2t",
                    punctuation: true,
                    normalization,
                    detofu_map: Some(&map),
                },
            );
            measure(&format!("pipeline_{mode_name}/{name}"), text, || {
                pipeline.convert(black_box(text))
            });
        }
        let pipeline = create_text_converter(
            &cc,
            TextConverterOptions {
                config: "s2t",
                punctuation: true,
                normalization: NormalizationMode::CompatExtended,
                detofu_map: Some(&custom),
            },
        );
        measure(&format!("pipeline_custom/{name}"), text, || {
            pipeline.convert(black_box(text))
        });
    }
    cc.set_parallel(true);
    for (name, text) in cases
        .iter()
        .filter(|(n, _)| *n == "short" || n.contains("novel"))
    {
        measure(&format!("parallel_convert/{name}"), text, || {
            cc.convert(black_box(text), "s2t", true)
        });
    }
}
