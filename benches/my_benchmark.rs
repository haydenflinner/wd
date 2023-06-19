use std::fs::File;
use bstr::ByteSlice;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use memmap::MmapOptions;
use wd::components::home::{get_visible_lines};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

// fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
// }

fn criterion_benchmark(c: &mut Criterion) {
    let file = File::open("./hugefile.txt").unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    // c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    // Used this to determine 3.7us vs 5.2 us for get_visible_lines returning a copy rather than a Cow, on a 4kb file.
    // Used this to determine 30us vs 57 us for get_visible_lines returning a copy rather than a Cow, on a 40kb file.
    // Not the bottleneck, go for it.
    c.bench_function("hugefile.txt getviz", |b| b.iter(|| get_visible_lines(black_box(mmap.as_bstr()), &vec!(), 10000, 10000)));
    // c.bench_function("hugefile.txt getviz copy", |b| b.iter(|| get_visible_lines_slow(black_box(mmap.as_bstr()), &vec!(), 10000, 10000)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);