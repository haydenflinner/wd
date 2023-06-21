use bstr::ByteSlice;
use std::fs::File;

use chrono::Local;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use memmap::MmapOptions;
use wd::components::home::get_visible_lines;

fn criterion_benchmark(c: &mut Criterion) {
    let file = File::open("./hugefile.txt").unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    // Used this to determine 3.7us vs 5.2 us for get_visible_lines returning a copy rather than a Cow, on a 4kb file.
    // Used this to determine 30us vs 57 us for get_visible_lines returning a copy rather than a Cow, on a 40kb file.
    // Not the bottleneck, go for it.
    c.bench_function("hugefile.txt getviz", |b| {
        b.iter(|| get_visible_lines(black_box(mmap.as_bstr()), &vec![], 10000, 10000, 0))
    });
    // c.bench_function("hugefile.txt getviz copy", |b| b.iter(|| get_visible_lines_slow(black_box(mmap.as_bstr()), &vec!(), 10000, 10000)));
    let s = "04/04/1997 12:04:01";
    // 40GB * (1.7microseconds / 311kb) ~= 200ms for a 40GB file ignoring cache misses, good enough.
    c.bench_function("dateparser generic", |b| {
        b.iter(|| dateparser::parse_with_timezone(black_box(s), &Local))
    });
    // 67ns to parse the epoch timestamp though, dang, must be nice.
    c.bench_function("dateparser generic epoch", |b| {
        b.iter(|| dateparser::parse_with_timezone(black_box("1687208330"), &Local))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
