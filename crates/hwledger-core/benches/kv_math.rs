use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn stub_bench(c: &mut Criterion) {
    c.bench_function("stub", |b| {
        b.iter(|| black_box(1 + 1))
    });
}

criterion_group!(benches, stub_bench);
criterion_main!(benches);
